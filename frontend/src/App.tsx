
import React, { useEffect, useRef } from 'react';
import { useSelector } from 'react-redux';
import { KeyboardEvent } from 'react';

import { RootState, useAppDispatch } from './app/store';
import { channelSlice } from './ui/channel';

const randomHash = () => {
  const arr = new Uint8Array(12);
  crypto.getRandomValues(arr);
  const hash = btoa([... arr].map(i => String.fromCodePoint(i)).join(''));
  return hash;
};

let ws: WebSocket | null = null;
let wsOpening = false;
let savedCanvasId = '';

type WsHandler = (bytes: Uint8Array) => void;
const wsHandlers = new Set<WsHandler>();
const addWsHandler = (handler: WsHandler) => {
  wsHandlers.add(handler);
};

const removeWsHandler = (handler: WsHandler) => {
  wsHandlers.delete(handler);
};

const openWs = () => {
  if (wsOpening) return;
  if (ws != null && ws.readyState < WebSocket.CLOSING) return;

  const maybeWs = new WebSocket('/ws');
  maybeWs.binaryType = "arraybuffer";
  wsOpening = true;

  maybeWs.onopen = () => {
    wsOpening = false;
    ws = maybeWs;
    console.log('ws: open');

    if (savedCanvasId == '') return;
    subscribe(savedCanvasId);
  };

  maybeWs.onclose = () => {
    wsOpening = false;
    ws = null;
    console.log('ws: close');

    if (document.visibilityState == 'hidden') {
      return;
    }

    setTimeout(() => openWs(), 100);
  };

  maybeWs.onerror = (ev) => {
    console.error(ev);
    if (ws != null && ws.readyState < WebSocket.CLOSING) return;

    ws = null;
    wsOpening = false;
    if (document.visibilityState == 'hidden') {
      return;
    }

    setTimeout(() => openWs(), 1000);
  };

  maybeWs.onmessage = (ev) => {
    const data = ev.data;
    if ('string' == typeof data) return;

    console.log('img received');
    wsHandlers.forEach((handler) => handler(new Uint8Array(data)));
  };
};

const syncData = (bytes: Blob) => {
  if (!ws) return;
  console.log('img sent');
  ws.send(bytes);
};

const subscribe = (canvas_id: string) => {
  savedCanvasId = canvas_id;
  if (!ws) return;
  console.log(`subscribed to ${canvas_id}`);
  ws.send(JSON.stringify({
    "type": "set_canvas",
    "canvas_id": canvas_id,
  }));
};

document.addEventListener('visibilitychange', () => {
  if (document.visibilityState == 'hidden') return;
  console.log('Page shown');
  openWs();
});

openWs();

export const App = () => {
  const canvas_wrapper = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const canvas_id = useSelector((state: RootState) => state.channel.canvas_id);
  const dispatch = useAppDispatch();

  useEffect(() => {
    const curHash = location.hash.slice(1);
    if (canvas_id == '' && curHash != '') {
      dispatch(channelSlice.actions.setCanvas({ canvas_id: curHash }));
    } else if (canvas_id == '') {
      const hash = randomHash();
      dispatch(channelSlice.actions.setCanvas({ canvas_id: hash }));
    }

    const changeHash = (hash: string) => {
      history.replaceState(null, '', `#${hash}`);
    };

    changeHash(canvas_id);
    document.title = `Odai: ${canvas_id}`;
    subscribe(canvas_id);

    const onHashChange = () => {
      const curHash = location.hash.slice(1);
      if (curHash == '') {
        changeHash(canvas_id);
      } else {
        dispatch(channelSlice.actions.setCanvas({ canvas_id: curHash }));
      }
    };

    window.addEventListener('hashchange', onHashChange);

    return () => {
      window.removeEventListener('hashchange', onHashChange);
    };
  }, [canvas_id]);

  useEffect(() =>  {
    const wrapperEl = canvas_wrapper.current;

    const drawLine = (x1: number, y1: number, x2: number, y2: number) => {
      const canvasEl = canvas.current;
      if (!canvasEl) return;
      const ratio = canvasEl.clientWidth / canvasEl.width;
      const ctx = canvasEl.getContext('2d');
      if (!ctx) return;
      ctx.lineWidth = 4;
      ctx.strokeStyle = "#000000";
      ctx.beginPath();
      ctx.moveTo(x1 / ratio, y1 / ratio);
      ctx.lineTo(x2 / ratio, y2 / ratio);
      ctx.stroke();
    };

    const sendUpdate = () => {
      console.log('sendUpdate()');
      const canvasEl = canvas.current;
      if (!canvasEl) return;
      canvasEl.toBlob((blob) => {
        if (!blob) return;
        syncData(blob);
      }, 'image/png');
    };

    const receiveData = (data: Uint8Array) => {
      const canvasEl = canvas.current;
      if (!canvasEl) return;
      const ctx = canvasEl.getContext('2d');
      if (!ctx) return;

      const blob = new Blob([data], { type: 'image/png' });
      const src = URL.createObjectURL(blob);
      const img = new Image();
      img.src = src;
      img.onload = () => {
        ctx.clearRect(0, 0, canvasEl.width, canvasEl.height);
        ctx.drawImage(img, 0, 0, canvasEl.width, canvasEl.height);
      };
    };

    if (wrapperEl) {
      let prevX = 0;
      let prevY = 0;
      let mouseIsDown = false;
      wrapperEl.onmousedown = (ev) => {
        prevX = ev.offsetX;
        prevY = ev.offsetY;
        mouseIsDown = true;
      };
      wrapperEl.onmouseout = (ev) => {
        if (mouseIsDown) {
          drawLine(prevX, prevY, ev.offsetX, ev.offsetY);
          sendUpdate();
        }
        mouseIsDown = false;
      };
      wrapperEl.onmousemove = (ev) => {
        if (!mouseIsDown) return;
        drawLine(prevX, prevY, ev.offsetX, ev.offsetY);
        prevX = ev.offsetX;
        prevY = ev.offsetY;
      };
      wrapperEl.onmouseup = (ev) => {
        if (!mouseIsDown) return;
        drawLine(prevX, prevY, ev.offsetX, ev.offsetY);
        sendUpdate();
        mouseIsDown = false;
      };
    }

    addWsHandler(receiveData);

    return () => {
      removeWsHandler(receiveData);

      const canvasEl = canvas.current;
      if (!canvasEl) return;
      const ctx = canvasEl.getContext('2d');
      ctx?.clearRect(0, 0, canvasEl.width, canvasEl.height);
    };
  }, [canvas_id, canvas, canvas_wrapper]);

  return <>
    <div id="canvas_id_wrapper">
      Channel ID:
      <input type="text" id="canvas_id" value={canvas_id} onChange={(e) => dispatch(channelSlice.actions.setCanvas({ canvas_id: (e.target as HTMLInputElement).value.trim() }))} autoComplete='off' />
      <button id="new_canvas" onClick={() => {
        const hash = randomHash();
        dispatch(channelSlice.actions.setCanvas({ canvas_id: hash }));
      }}>New…</button>
    </div>
    <div ref={canvas_wrapper} id="canvas_wrapper">
      <canvas ref={canvas} id="canvas" width="1920" height="1920"></canvas>
    </div>
  </>;
};


import { createSlice } from '@reduxjs/toolkit';


const INITIAL_STATE: { canvas_id: string } = {
  canvas_id: '',
};

export const channelSlice = createSlice({
  name: 'channel',
  initialState: INITIAL_STATE,
  reducers: {
    setCanvas: (state, action) => {
      state.canvas_id = action.payload.canvas_id;
    },
  },
});

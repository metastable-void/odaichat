import { configureStore } from '@reduxjs/toolkit';

import { channelSlice } from '../ui/channel';
import { useDispatch } from 'react-redux';

export const store = configureStore({
  reducer: {
    channel: channelSlice.reducer,
  }
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;

export const useAppDispatch = useDispatch.withTypes<AppDispatch>()
export default store;

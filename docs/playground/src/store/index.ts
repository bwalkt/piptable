import { effect } from '@preact/signals';
import {
  code,
  error,
  examples,
  isRunning,
  output,
  runCode,
  selectExample,
  selectedExample,
  setTheme,
  theme
} from './playground';

export { examples };

type PlaygroundState = {
  code: string;
  output: string;
  error: string | null;
  isRunning: boolean;
  selectedExample: string;
  theme: 'light' | 'dark';
  setCode: (value: string) => void;
  runCode: () => Promise<void>;
  selectExample: (name: string) => void;
  setTheme: (value: 'light' | 'dark') => void;
};

const getState = (): PlaygroundState => ({
  code: code.value,
  output: output.value,
  error: error.value,
  isRunning: isRunning.value,
  selectedExample: selectedExample.value,
  theme: theme.value,
  setCode: (value: string) => {
    code.value = value;
  },
  runCode,
  selectExample,
  setTheme
});

type Listener = () => void;

const subscribe = (listener: Listener) => {
  const dispose = effect(() => {
    code.value;
    output.value;
    error.value;
    isRunning.value;
    selectedExample.value;
    theme.value;
    listener();
  });

  return () => {
    dispose();
  };
};

export const usePlaygroundStore = {
  getState,
  subscribe
};

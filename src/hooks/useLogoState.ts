import { useState, useCallback, useRef, useEffect } from 'react';

export type LogoState = 'normal' | 'happy' | 'working' | 'error' | 'thinking';

interface UseLogoStateOptions {
  defaultState?: LogoState;
  autoReturnToNormal?: boolean;
  autoReturnDelay?: number;
}

export const useLogoState = (options: UseLogoStateOptions = {}) => {
  const {
    defaultState = 'normal',
    autoReturnToNormal = true,
    autoReturnDelay = 2000
  } = options;

  const [state, setState] = useState<LogoState>(defaultState);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  // 清理定时器
  const clearAutoReturn = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  // 设置状态并可选自动返回
  const setLogoState = useCallback((newState: LogoState, temporary = true) => {
    clearAutoReturn();
    setState(newState);

    if (temporary && autoReturnToNormal && newState !== defaultState) {
      timeoutRef.current = setTimeout(() => {
        setState(defaultState);
      }, autoReturnDelay);
    }
  }, [defaultState, autoReturnToNormal, autoReturnDelay, clearAutoReturn]);

  // 便捷方法
  const showHappy = useCallback(() => setLogoState('happy'), [setLogoState]);
  const showWorking = useCallback(() => setLogoState('working', false), [setLogoState]);
  const showError = useCallback(() => setLogoState('error'), [setLogoState]);
  const showThinking = useCallback(() => setLogoState('thinking', false), [setLogoState]);
  const showNormal = useCallback(() => setLogoState('normal', false), [setLogoState]);

  // 链式动画支持
  const playSequence = useCallback(async (sequence: { state: LogoState; duration: number }[]) => {
    clearAutoReturn();
    
    for (const { state: seqState, duration } of sequence) {
      setState(seqState);
      await new Promise(resolve => setTimeout(resolve, duration));
    }
    
    setState(defaultState);
  }, [defaultState, clearAutoReturn]);

  // 清理
  useEffect(() => {
    return () => clearAutoReturn();
  }, [clearAutoReturn]);

  return {
    state,
    setLogoState,
    showHappy,
    showWorking,
    showError,
    showThinking,
    showNormal,
    playSequence,
    clearAutoReturn
  };
};
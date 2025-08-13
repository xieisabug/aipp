import { useState, useEffect, useCallback } from 'react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

export const useCopyHandler = (content: string) => {
    const [copyIconState, setCopyIconState] = useState<'copy' | 'ok'>('copy');

    const handleCopy = useCallback(() => {
        writeText(content);
        setCopyIconState('ok');
    }, [content]);

    useEffect(() => {
        if (copyIconState === 'ok') {
            const timer = setTimeout(() => {
                setCopyIconState('copy');
            }, 1500);

            return () => clearTimeout(timer);
        }
    }, [copyIconState]);

    return {
        copyIconState,
        handleCopy,
    };
};
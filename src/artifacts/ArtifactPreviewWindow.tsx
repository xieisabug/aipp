import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

interface LogLine {
    type: 'log' | 'error' | 'success';
    message: string;
}

/**
 * 仅用于 "artifact_preview" 窗口。
 * - 监听后端发出的 artifact-log / artifact-error / artifact-success 事件并展示。
 * - 监听 artifact-redirect 事件并在收到后跳转到相应 URL。
 */
export default function ArtifactPreviewWindow() {
    const [logs, setLogs] = useState<LogLine[]>([]);
    const logsEndRef = useRef<HTMLDivElement | null>(null);
    const unlistenersRef = useRef<(() => void)[]>([]);
    const isRegisteredRef = useRef(false);

    // 自动滚动到底部
    useEffect(() => {
        logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [logs]);

    // 注册事件监听
    useEffect(() => {
        // 在函数执行一开始就检查并设置标志位，避免竞争条件
        if (isRegisteredRef.current) {
            return;
        }
        isRegisteredRef.current = true;

        const registerListeners = async () => {
            const addLog = (type: LogLine['type']) => (event: { payload: any }) => {
                console.log('🔧 [ArtifactPreviewWindow] 添加日志', event.payload);
                setLogs(prev => [...prev, { type, message: event.payload as string }]);
            };

            console.log('🔧 [ArtifactPreviewWindow] 注册事件监听');

            const unlisteners = await Promise.all([
                listen('artifact-log', addLog('log')),
                listen('artifact-error', addLog('error')),
                listen('artifact-success', addLog('success')),
                listen('artifact-redirect', (event) => {
                    const url = event.payload as string;
                    window.location.href = url;
                })
            ]);

            unlistenersRef.current = unlisteners;
        };

        registerListeners();

        return () => {
            console.log('🔧 [ArtifactPreviewWindow] 卸载事件监听');
            unlistenersRef.current.forEach((fn) => fn());
            unlistenersRef.current = [];
            isRegisteredRef.current = false;
        };
    }, []);

    // 监听窗口关闭事件，清理预览服务器
    useEffect(() => {
        const currentWindow = getCurrentWebviewWindow();

        const cleanup = async () => {
            try {
                // 调用后端API关闭React预览服务器
                await invoke('close_react_preview', { previewId: 'react' });
                setLogs([]);
                console.log('Preview server cleanup completed');
            } catch (error) {
                console.error('Failed to cleanup preview server:', error);
            }
        };

        // 监听窗口关闭事件
        const unlistenCloseRequested = currentWindow.onCloseRequested(cleanup);

        return () => {
            unlistenCloseRequested.then(fn => fn());
        };
    }, []);

    return (
        <div className="w-full h-full flex flex-col p-4">
            <h2 className="text-lg font-semibold mb-2">Artifact Preview Logs</h2>
            <div className="flex-1 overflow-y-auto rounded border p-2 bg-gray-50 text-sm font-mono">
                {logs.map((log, idx) => (
                    <div
                        key={idx}
                        className={
                            log.type === 'error'
                                ? 'text-red-600'
                                : log.type === 'success'
                                    ? 'text-green-700'
                                    : 'text-gray-800'
                        }
                    >
                        {log.message}
                    </div>
                ))}
                <div ref={logsEndRef} />
            </div>
        </div>
    );
} 
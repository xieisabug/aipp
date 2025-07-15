import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import '../styles/ArtifactPreviewWIndow.css';

interface LogLine {
    type: 'log' | 'error' | 'success';
    message: string;
}

/**
 * 仅用于 "artifact_preview" 窗口。
 * - 监听后端发出的 artifact-log / artifact-error / artifact-success 事件并展示。
 * - 使用 iframe 沙盒展示预览内容，避免页面跳转导致监听器失效。
 * - 显示模式：先显示日志，预览准备好后切换到全屏预览
 */
export default function ArtifactPreviewWindow() {
    const [logs, setLogs] = useState<LogLine[]>([]);
    const [previewUrl, setPreviewUrl] = useState<string | null>(null);
    const [isPreviewReady, setIsPreviewReady] = useState(false);
    const [currentView, setCurrentView] = useState<'logs' | 'preview'>('logs');
    const logsEndRef = useRef<HTMLDivElement | null>(null);
    const unlistenersRef = useRef<(() => void)[]>([]);
    const isRegisteredRef = useRef(false);

    // 自动滚动到底部
    useEffect(() => {
        logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [logs]);

    // 当预览准备好时，切换到预览视图
    useEffect(() => {
        if (isPreviewReady && previewUrl) {
            // 延迟一秒再切换，让用户看到"预览准备完成"的日志
            const timer = setTimeout(() => {
                setCurrentView('preview');
            }, 300);
            return () => clearTimeout(timer);
        }
    }, [isPreviewReady, previewUrl]);

    // 注册事件监听
    useEffect(() => {
        let isCancelled = false;

        const registerListeners = async () => {
            // 在函数执行一开始就检查并设置标志位，避免竞争条件
            if (isRegisteredRef.current || isCancelled) {
                return;
            }
            isRegisteredRef.current = true;

            const addLog = (type: LogLine['type']) => (event: { payload: any }) => {
                console.log('🔧 [ArtifactPreviewWindow] 添加日志', event.payload);
                setLogs(prev => [...prev, { type, message: event.payload as string }]);
            };

            const handleRedirect = (event: { payload: any }) => {
                const url = event.payload as string;
                console.log('🔧 [ArtifactPreviewWindow] 收到预览 URL:', url);
                setPreviewUrl(url);
                setIsPreviewReady(true);
            };

            console.log('🔧 [ArtifactPreviewWindow] 注册事件监听');

            try {
                const unlisteners = await Promise.all([
                    listen('artifact-log', addLog('log')),
                    listen('artifact-error', addLog('error')),
                    listen('artifact-success', addLog('success')),
                    listen('artifact-redirect', handleRedirect)
                ]);

                // 检查是否已被取消
                if (isCancelled) {
                    unlisteners.forEach((fn) => fn());
                    return;
                }

                unlistenersRef.current = unlisteners;
            } catch (error) {
                console.error('注册事件监听失败:', error);
                isRegisteredRef.current = false;
            }
        };

        registerListeners();

        return () => {
            console.log('🔧 [ArtifactPreviewWindow] 卸载事件监听');
            isCancelled = true;
            unlistenersRef.current.forEach((fn) => fn());
            unlistenersRef.current = [];
            isRegisteredRef.current = false;
        };
    }, []);

    // 监听窗口关闭事件，清理预览服务器
    useEffect(() => {
        const currentWindow = getCurrentWebviewWindow();
        let unlistenCloseRequested: (() => void) | null = null;
        let isCleanupDone = false;

        const cleanup = async () => {
            // 避免重复清理
            if (isCleanupDone) return;
            isCleanupDone = true;

            try {
                console.log('🔧 [ArtifactPreviewWindow] 窗口关闭，开始清理预览服务器');
                // 调用后端API关闭React预览服务器
                await invoke('close_react_preview', { previewId: 'react' });

                setLogs([]);
                setPreviewUrl(null);
                setIsPreviewReady(false);
                setCurrentView('logs');

                console.log('🔧 [ArtifactPreviewWindow] 预览服务器清理完成');
            } catch (error) {
                console.error('🔧 [ArtifactPreviewWindow] 清理预览服务器失败:', error);
            }
        };

        // 监听窗口关闭事件 - Tauri v2 的正确用法
        const setupCloseListener = async () => {
            try {
                unlistenCloseRequested = await currentWindow.onCloseRequested(cleanup);
                console.log('🔧 [ArtifactPreviewWindow] 窗口关闭监听器已注册');
            } catch (error) {
                console.error('🔧 [ArtifactPreviewWindow] 注册窗口关闭监听器失败:', error);
            }
        };

        setupCloseListener();

        // 添加组件卸载时的清理
        return () => {
            if (unlistenCloseRequested) {
                unlistenCloseRequested();
                console.log('🔧 [ArtifactPreviewWindow] 窗口关闭监听器已移除');
            }
            // 组件卸载时也执行清理
            if (!isCleanupDone) {
                cleanup();
            }
        };
    }, []);

    // 添加切换视图的按钮（可选）
    const handleToggleView = () => {
        setCurrentView(current => current === 'logs' ? 'preview' : 'logs');
    };

    return (
        <div className="w-full h-full flex flex-col">
            {/* 顶部工具栏 */}
            {isPreviewReady && previewUrl && (
                <div className="flex-shrink-0 p-2 bg-gray-100 border-b flex items-center justify-between">
                    <div className="text-sm text-gray-600">
                        {currentView === 'logs' ? '日志视图' : `预览地址: ${previewUrl}`}
                    </div>
                    <button
                        onClick={handleToggleView}
                        className="px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600"
                    >
                        {currentView === 'logs' ? '查看预览' : '查看日志'}
                    </button>
                </div>
            )}

            {/* 主要内容区域 */}
            <div className="flex-1 flex flex-col">
                {currentView === 'logs' ? (
                    /* 日志视图 - 全屏显示 */
                    <div className="flex-1 flex flex-col p-4">
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
                        
                        {/* 如果预览准备好了，显示提示 */}
                        {isPreviewReady && previewUrl && (
                            <div className="mt-4 p-3 bg-green-50 border border-green-200 rounded">
                                <p className="text-green-700 text-sm">
                                    ✅ 预览准备完成，即将自动切换到预览视图...
                                </p>
                            </div>
                        )}
                    </div>
                ) : (
                    /* 预览视图 - 全屏 iframe */
                    <div className="flex-1 flex flex-col">
                        <iframe
                            src={previewUrl || ''}
                            className="flex-1 w-full border-0"
                            sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-presentation"
                            onLoad={() => {
                                console.log('🔧 [ArtifactPreviewWindow] iframe 加载完成');
                            }}
                            onError={(e) => {
                                console.error('🔧 [ArtifactPreviewWindow] iframe 加载失败:', e);
                            }}
                        />
                    </div>
                )}
            </div>
        </div>
    );
} 
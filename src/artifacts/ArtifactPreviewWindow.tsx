import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { open } from '@tauri-apps/plugin-shell';
import mermaid from 'mermaid';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';
import remarkMath from 'remark-math';
import remarkBreaks from 'remark-breaks';
import rehypeKatex from 'rehype-katex';
import rehypeRaw from 'rehype-raw';
import '../styles/ArtifactPreviewWIndow.css';
import 'katex/dist/katex.min.css';

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
    const [previewType, setPreviewType] = useState<'react' | 'vue' | 'mermaid' | 'html' | 'svg' | 'xml' | 'markdown' | 'md' | null>(null);
    const logsEndRef = useRef<HTMLDivElement | null>(null);
    const unlistenersRef = useRef<(() => void)[]>([]);
    const isRegisteredRef = useRef(false);
    const previewTypeRef = useRef<'react' | 'vue' | 'mermaid' | 'html' | 'svg' | 'xml' | 'markdown' | 'md' | null>(null);
    const mermaidContainerRef = useRef<HTMLDivElement | null>(null);
    const [mermaidContent, setMermaidContent] = useState<string>('');
    const [htmlContent, setHtmlContent] = useState<string>('');
    const [markdownContent, setMarkdownContent] = useState<string>('');
    const [mermaidScale, setMermaidScale] = useState<number>(1);
    const [mermaidPosition, setMermaidPosition] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
    const [isDragging, setIsDragging] = useState<boolean>(false);
    const [dragStart, setDragStart] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
    const [isSpacePressed, setIsSpacePressed] = useState<boolean>(false);

    // 同步 previewType 到 ref
    useEffect(() => {
        previewTypeRef.current = previewType;
    }, [previewType]);

    // 初始化 mermaid
    useEffect(() => {
        mermaid.initialize({
            startOnLoad: false,
            theme: 'default',
            securityLevel: 'loose',
            fontFamily: 'monospace'
        });
    }, []);

    // 自动滚动到底部
    useEffect(() => {
        logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [logs]);

    // 渲染 mermaid 图表
    useEffect(() => {

        // 确保在预览视图且是 mermaid 类型时才渲染
        if (previewType === 'mermaid' && currentView === 'preview' && mermaidContent && mermaidContainerRef.current) {
            const renderMermaid = async () => {
                try {
                    const container = mermaidContainerRef.current;
                    if (!container) return;

                    // 找到内部的可缩放容器
                    const innerContainer = container.querySelector('div > div') as HTMLDivElement;
                    if (!innerContainer) return;

                    // 清空容器
                    innerContainer.innerHTML = '';

                    // 创建一个唯一的ID
                    const id = `mermaid-${Date.now()}`;

                    // 验证 mermaid 内容
                    if (!mermaidContent.trim()) {
                        innerContainer.innerHTML = '<div class="text-red-500 p-4">Mermaid 内容为空</div>';
                        return;
                    }

                    // 渲染图表
                    const { svg } = await mermaid.render(id, mermaidContent.trim());
                    innerContainer.innerHTML = svg;

                    // 设置 SVG 样式以适应容器
                    const svgElement = innerContainer.querySelector('svg');
                    if (svgElement) {
                        svgElement.style.maxWidth = 'none';
                        svgElement.style.maxHeight = 'none';
                        svgElement.style.width = 'auto';
                        svgElement.style.height = 'auto';
                    }
                } catch (error) {
                    const container = mermaidContainerRef.current;
                    if (container) {
                        const innerContainer = container.querySelector('div > div') as HTMLDivElement;
                        if (innerContainer) {
                            innerContainer.innerHTML = `<div class="text-red-500 p-4">渲染失败: ${error}</div>`;
                        }
                    }
                }
            };

            // 延迟渲染，确保 DOM 已准备好
            setTimeout(renderMermaid, 200);
        }
    }, [previewType, currentView, mermaidContent]);

    // 处理Mermaid图表的交互事件
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.code === 'Space' && previewType === 'mermaid' && currentView === 'preview') {
                e.preventDefault();
                setIsSpacePressed(true);
            }
        };

        const handleKeyUp = (e: KeyboardEvent) => {
            if (e.code === 'Space') {
                setIsSpacePressed(false);
                setIsDragging(false);
            }
        };

        const handleWheel = (e: WheelEvent) => {
            if (previewType === 'mermaid' && currentView === 'preview' && mermaidContainerRef.current?.contains(e.target as Node)) {
                e.preventDefault();
                const delta = e.deltaY > 0 ? -0.1 : 0.1;
                setMermaidScale(prevScale => Math.max(0.1, Math.min(3, prevScale + delta)));
            }
        };

        document.addEventListener('keydown', handleKeyDown);
        document.addEventListener('keyup', handleKeyUp);
        document.addEventListener('wheel', handleWheel, { passive: false });

        return () => {
            document.removeEventListener('keydown', handleKeyDown);
            document.removeEventListener('keyup', handleKeyUp);
            document.removeEventListener('wheel', handleWheel);
        };
    }, [previewType, currentView]);

    // 处理鼠标拖动
    const handleMouseDown = (e: React.MouseEvent) => {
        if (isSpacePressed && previewType === 'mermaid') {
            setIsDragging(true);
            setDragStart({ x: e.clientX - mermaidPosition.x, y: e.clientY - mermaidPosition.y });
        }
    };

    const handleMouseMove = (e: React.MouseEvent) => {
        if (isDragging && isSpacePressed) {
            setMermaidPosition({
                x: e.clientX - dragStart.x,
                y: e.clientY - dragStart.y
            });
        }
    };

    const handleMouseUp = () => {
        setIsDragging(false);
    };

    // 重置Mermaid缩放和位置
    const resetMermaidView = () => {
        setMermaidScale(1);
        setMermaidPosition({ x: 0, y: 0 });
    };

    // 当预览准备好时，切换到预览视图
    useEffect(() => {
        if (isPreviewReady && (previewUrl || previewType === 'mermaid' || previewType === 'html' || previewType === 'svg' || previewType === 'xml' || previewType === 'markdown' || previewType === 'md')) {
            setCurrentView('preview');
        }
    }, [isPreviewReady, previewUrl, previewType]);

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
                const message = event.payload as string;
                setLogs(prev => [...prev, { type, message }]);

                // 根据日志内容检测预览类型
                if (message.includes('Vue') || message.includes('vue')) {
                    setPreviewType('vue');
                } else if (message.includes('React') || message.includes('react')) {
                    setPreviewType('react');
                } else if (message.includes('Mermaid') || message.includes('mermaid')) {
                    setPreviewType('mermaid');
                    // 如果是 mermaid，从日志中提取内容
                    const mermaidMatch = message.match(/mermaid content: ([\s\S]+)/);
                    if (mermaidMatch && mermaidMatch[1]) {
                        setMermaidContent(mermaidMatch[1]);
                        setIsPreviewReady(true);
                    }
                } else if (message.includes('html content:')) {
                    setPreviewType('html');
                    const htmlMatch = message.match(/html content: ([\s\S]+)/);
                    if (htmlMatch && htmlMatch[1]) {
                        setHtmlContent(htmlMatch[1]);
                        setIsPreviewReady(true);
                    }
                } else if (message.includes('svg content:')) {
                    setPreviewType('svg');
                    const svgMatch = message.match(/svg content: ([\s\S]+)/);
                    if (svgMatch && svgMatch[1]) {
                        setHtmlContent(svgMatch[1]);
                        setIsPreviewReady(true);
                    }
                } else if (message.includes('xml content:')) {
                    setPreviewType('xml');
                    const xmlMatch = message.match(/xml content: ([\s\S]+)/);
                    if (xmlMatch && xmlMatch[1]) {
                        setHtmlContent(xmlMatch[1]);
                        setIsPreviewReady(true);
                    }
                } else if (message.includes('markdown content:') || message.includes('md content:')) {
                    const type = message.includes('markdown content:') ? 'markdown' : 'md';
                    setPreviewType(type);
                    const contentMatch = message.match(/(markdown|md) content: ([\s\S]+)/);
                    if (contentMatch && contentMatch[2]) {
                        setMarkdownContent(contentMatch[2]);
                        setIsPreviewReady(true);
                    }
                }
            };

            const handleRedirect = (event: { payload: any }) => {
                const url = event.payload as string;
                setPreviewUrl(url);
                setIsPreviewReady(true);
            };


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
                isRegisteredRef.current = false;
            }
        };

        registerListeners();

        return () => {
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
                // 根据预览类型调用相应的关闭函数
                if (previewTypeRef.current === 'vue') {
                    await invoke('close_vue_preview', { previewId: 'vue' });
                } else if (previewTypeRef.current === 'mermaid' || previewTypeRef.current === 'html' || previewTypeRef.current === 'svg' || previewTypeRef.current === 'xml' || previewTypeRef.current === 'markdown' || previewTypeRef.current === 'md') {
                    // Mermaid/HTML/SVG/XML/Markdown 不需要服务器清理，只需要清除DOM
                } else {
                    await invoke('close_react_preview', { previewId: 'react' });
                }

                setLogs([]);
                setPreviewUrl(null);
                setIsPreviewReady(false);
                setCurrentView('logs');
                setPreviewType(null);
                setMermaidContent('');
                setHtmlContent('');
                setMarkdownContent('');

            } catch (error) {
            }
        };

        // 监听窗口关闭事件 - Tauri v2 的正确用法
        const setupCloseListener = async () => {
            try {
                unlistenCloseRequested = await currentWindow.onCloseRequested(cleanup);
            } catch (error) {
            }
        };

        setupCloseListener();

        // 添加组件卸载时的清理
        return () => {
            if (unlistenCloseRequested) {
                unlistenCloseRequested();
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

    // 在浏览器中打开预览页面
    const handleOpenInBrowser = async () => {
        if (previewUrl) {
            try {
                await open(previewUrl);
            } catch (error) {
            }
        }
    };

    // 刷新iframe
    const handleRefresh = () => {
        if (previewUrl) {
            // 移除现有的_refresh参数，然后添加新的时间戳
            const url = new URL(previewUrl);
            url.searchParams.set('_refresh', Date.now().toString());
            setPreviewUrl(url.toString());
        }
    };

    return (
        <div className="flex h-screen bg-gray-100">
            <div className="flex flex-col flex-1 bg-white rounded-xl m-2 shadow-lg">
                {/* 顶部工具栏 */}
                {isPreviewReady && (previewUrl || previewType === 'mermaid' || previewType === 'html' || previewType === 'svg' || previewType === 'xml' || previewType === 'markdown' || previewType === 'md') && (
                    <div className="flex-shrink-0 p-4 border-b flex items-center justify-between">
                        <div className="text-sm text-gray-600">
                            {currentView === 'logs' ? '日志视图' :
                                previewType === 'mermaid' ? 'Mermaid 图表预览' :
                                    previewType === 'html' ? 'HTML 预览' :
                                        previewType === 'svg' ? 'SVG 预览' :
                                            previewType === 'xml' ? 'XML 预览' :
                                                previewType === 'markdown' || previewType === 'md' ? 'Markdown 预览' :
                                                    `预览地址: ${previewUrl}`}
                        </div>
                        <div className="flex gap-2">
                            {previewType !== 'mermaid' && previewType !== 'html' && previewType !== 'svg' && previewType !== 'xml' && previewType !== 'markdown' && previewType !== 'md' && (
                                <>
                                    <button
                                        onClick={handleRefresh}
                                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white shadow-md hover:shadow-lg transition-all rounded-md text-sm font-medium"
                                        title="刷新预览"
                                    >
                                        刷新
                                    </button>
                                    <button
                                        onClick={handleOpenInBrowser}
                                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white shadow-md hover:shadow-lg transition-all rounded-md text-sm font-medium"
                                        title="在浏览器中打开"
                                    >
                                        打开浏览器
                                    </button>
                                </>
                            )}
                            <button
                                onClick={handleToggleView}
                                className="px-6 py-2 bg-gray-800 hover:bg-gray-900 text-white shadow-md hover:shadow-lg transition-all rounded-md text-sm font-medium"
                            >
                                {currentView === 'logs' ? '查看预览' : '查看日志'}
                            </button>
                        </div>
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
                            {isPreviewReady && (previewUrl || previewType === 'mermaid' || previewType === 'html' || previewType === 'svg' || previewType === 'xml' || previewType === 'markdown' || previewType === 'md') && (
                                <div className="mt-4 p-3 bg-green-50 border border-green-200 rounded">
                                    <p className="text-green-700 text-sm">
                                        ✅ 预览准备完成，即将自动切换到预览视图...
                                    </p>
                                </div>
                            )}
                        </div>
                    ) : (
                        /* 预览视图 - 根据类型显示不同内容 */
                        <div className="flex-1 flex flex-col">
                            {previewType === 'mermaid' ? (
                                /* Mermaid 图表预览 */
                                <div className="flex-1 flex flex-col p-4">
                                    <div className="flex justify-between items-center mb-2">
                                        <div className="text-sm text-gray-600">
                                            缩放: {Math.round(mermaidScale * 100)}% | 提示: 滚轮缩放，空格键+拖动
                                        </div>
                                        <button
                                            onClick={resetMermaidView}
                                            className="px-3 py-1 bg-gray-600 hover:bg-gray-700 text-white text-xs rounded transition-colors"
                                        >
                                            重置视图
                                        </button>
                                    </div>
                                    <div
                                        ref={mermaidContainerRef}
                                        className={`flex-1 bg-white border rounded-lg shadow-sm overflow-hidden relative ${
                                            isSpacePressed ? 'cursor-grab' : 'cursor-default'
                                        } ${isDragging ? 'cursor-grabbing' : ''}`}
                                        onMouseDown={handleMouseDown}
                                        onMouseMove={handleMouseMove}
                                        onMouseUp={handleMouseUp}
                                        onMouseLeave={handleMouseUp}
                                        style={{
                                            minHeight: '400px',
                                            maxHeight: 'calc(100vh - 200px)',
                                            overflow: 'auto'
                                        }}
                                    >
                                        <div
                                            style={{
                                                transform: `scale(${mermaidScale}) translate(${mermaidPosition.x}px, ${mermaidPosition.y}px)`,
                                                transformOrigin: 'center center',
                                                transition: isDragging ? 'none' : 'transform 0.1s ease-out',
                                                display: 'flex',
                                                justifyContent: 'center',
                                                alignItems: 'center',
                                                minWidth: '100%',
                                                minHeight: '100%',
                                                padding: '20px'
                                            }}
                                        >
                                            {/* Mermaid SVG 将被渲染在这里 */}
                                        </div>
                                    </div>
                                </div>
                            ) : previewType === 'markdown' || previewType === 'md' ? (
                                /* Markdown 预览 */
                                <div className="flex-1 overflow-auto bg-white p-6">
                                    <div className="prose prose-lg max-w-none dark:prose-invert">
                                        <ReactMarkdown
                                            remarkPlugins={[remarkMath, remarkBreaks]}
                                            rehypePlugins={[rehypeKatex, rehypeRaw]}
                                            components={{
                                                code({ className, children, ...props }: any) {
                                                    const match = /language-(\w+)/.exec(className || '');
                                                    const isInline = !match;
                                                    return !isInline ? (
                                                        <SyntaxHighlighter
                                                            style={oneDark as any}
                                                            language={match[1]}
                                                            PreTag="div"
                                                            {...props}
                                                        >
                                                            {String(children).replace(/\n$/, '')}
                                                        </SyntaxHighlighter>
                                                    ) : (
                                                        <code className={className} {...props}>
                                                            {children}
                                                        </code>
                                                    );
                                                }
                                            }}
                                        >
                                            {markdownContent}
                                        </ReactMarkdown>
                                    </div>
                                </div>
                            ) : previewType === 'html' || previewType === 'svg' || previewType === 'xml' ? (
                                /* HTML/SVG/XML 预览 */
                                <iframe
                                    srcDoc={htmlContent}
                                    className="flex-1 w-full border-0"
                                    sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-presentation"
                                    style={{
                                        minHeight: '400px',
                                        backgroundColor: 'white'
                                    }}
                                />
                            ) : (
                                /* iframe 预览 - 用于 React 和 Vue */
                                <iframe
                                    src={previewUrl || ''}
                                    className="flex-1 w-full border-0"
                                    sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-presentation"
                                    onLoad={() => {
                                    }}
                                    onError={() => {
                                    }}
                                />
                            )}
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
} 
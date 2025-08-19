import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
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
import EnvironmentInstallDialog from '../components/EnvironmentInstallDialog';
import { useTheme } from '../hooks/useTheme';

interface LogLine {
    type: 'log' | 'error' | 'success';
    message: string;
}

/**
 * 仅用于 "artifact" 窗口。
 * - 监听后端发出的 artifact-log / artifact-error / artifact-success 事件并展示。
 * - 使用 iframe 沙盒展示预览内容，避免页面跳转导致监听器失效。
 * - 显示模式：先显示加载界面，预览准备好后切换到全屏预览
 */
export default function ArtifactWindow() {
    // 集成主题系统
    useTheme();

    const [logs, setLogs] = useState<LogLine[]>([]);
    const [previewUrl, setPreviewUrl] = useState<string | null>(null);
    const [isPreviewReady, setIsPreviewReady] = useState(false);
    const [currentView, setCurrentView] = useState<'loading' | 'preview'>('loading');
    const [previewType, setPreviewType] = useState<'react' | 'vue' | 'mermaid' | 'html' | 'svg' | 'xml' | 'markdown' | 'md' | null>(null);
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
    const isInstalling = useRef<boolean>(false);
    
    // 环境安装相关状态
    const [showEnvironmentDialog, setShowEnvironmentDialog] = useState<boolean>(false);
    const [environmentTool, setEnvironmentTool] = useState<string>('');
    const [environmentMessage, setEnvironmentMessage] = useState<string>('');
    const [currentLang, setCurrentLang] = useState<string>('');
    const [currentInputStr, setCurrentInputStr] = useState<string>('');
    
    // 使用 refs 来存储最新的值，避免闭包陷阱
    const currentLangRef = useRef<string>('');
    const currentInputStrRef = useRef<string>('');

    // 同步 previewType 到 ref
    useEffect(() => {
        previewTypeRef.current = previewType;
    }, [previewType]);

    // 同步 currentLang 和 currentInputStr 到 refs
    useEffect(() => {
        currentLangRef.current = currentLang;
        currentInputStrRef.current = currentInputStr;
    }, [currentLang, currentInputStr]);

    // 初始化 mermaid - 根据主题动态配置
    useEffect(() => {
        // 检测当前主题
        const isDark = document.documentElement.classList.contains('dark');
        
        mermaid.initialize({
            startOnLoad: false,
            theme: isDark ? 'dark' : 'default',
            securityLevel: 'loose',
            fontFamily: 'monospace',
            themeVariables: {
                darkMode: isDark,
            }
        });
    }, []);

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

    // 处理环境安装确认
    const handleEnvironmentInstallConfirm = async () => {
        try {
            await invoke('confirm_environment_install', {
                tool: environmentTool,
                confirmed: true,
                lang: currentLangRef.current,
                inputStr: currentInputStrRef.current
            });
        } catch (error) {
            setLogs(prev => [...prev, { type: 'error', message: `确认安装失败: ${error}` }]);
        }
    };

    // 处理环境安装取消
    const handleEnvironmentInstallCancel = async () => {
        try {
            await invoke('confirm_environment_install', {
                tool: environmentTool,
                confirmed: false,
                lang: currentLangRef.current,
                inputStr: currentInputStrRef.current
            });
            setShowEnvironmentDialog(false);
        } catch (error) {
            setLogs(prev => [...prev, { type: 'error', message: `取消安装失败: ${error}` }]);
        }
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
                console.log("[ArtifactWindow] 接收消息：", message);
                setLogs(prev => [...prev, { type, message }]);
            };

            const handleArtifactData = (event: { payload: any }) => {
                const data = event.payload;
                if (data.original_code && data.type) {
                    switch (data.type) {
                        case 'vue':
                        case 'react':
                            setPreviewType(data.type);
                            break;
                        case 'mermaid':
                            setPreviewType('mermaid');
                            setMermaidContent(data.original_code);
                            setIsPreviewReady(true);
                            break;
                        case 'html':
                            setPreviewType('html');
                            setHtmlContent(data.original_code);
                            setIsPreviewReady(true);
                            break;
                        case 'svg':
                            setPreviewType('svg');
                            setHtmlContent(data.original_code);
                            setIsPreviewReady(true);
                            break;
                        case 'xml':
                            setPreviewType('xml');
                            setHtmlContent(data.original_code);
                            setIsPreviewReady(true);
                            break;
                        case 'markdown':
                        case 'md':
                            setPreviewType(data.type);
                            setMarkdownContent(data.original_code);
                            setIsPreviewReady(true);
                            break;
                        default:
                            break;
                    }
                    // 删除原来的 setOriginalCode 调用
                }
            };

            const handleRedirect = (event: { payload: any }) => {
                const url = event.payload as string;
                setPreviewUrl(url);
                setIsPreviewReady(true);
            };

            const handleEnvironmentCheck = (event: { payload: any }) => {
                const data = event.payload;
                setEnvironmentTool(data.tool);
                setEnvironmentMessage(data.message);
                setCurrentLang(data.lang);
                setCurrentInputStr(data.input_str);
                setShowEnvironmentDialog(true);
            };

            const handleEnvironmentInstallStarted = (event: { payload: any }) => {
                const data = event.payload;
                setCurrentLang(data.lang);
                setCurrentInputStr(data.input_str);
                isInstalling.current = true;
                setShowEnvironmentDialog(false);
            };

            const handleBunInstallFinished = (event: { payload: any }) => {
                const success = event.payload as boolean;
                console.log('🔧 [ArtifactPreviewWindow] 收到Bun安装完成事件:', success, isInstalling);
                if (success && isInstalling.current) {
                    setLogs(prev => [...prev, { type: 'success', message: 'Bun 安装成功，正在重新启动预览...' }]);
                    // 重新启动预览
                    invoke('retry_preview_after_install', {
                        lang: currentLangRef.current,
                        inputStr: currentInputStrRef.current
                    }).then(() => {
                        isInstalling.current = false;
                    }).catch(error => {
                        setLogs(prev => [...prev, { type: 'error', message: `重新启动预览失败: ${error}` }]);
                        isInstalling.current = false;
                    });
                } else if (!success) {
                    setLogs(prev => [...prev, { type: 'error', message: 'Bun 安装失败' }]);
                    isInstalling.current = false;
                }
            };

            const handleUvInstallFinished = (event: { payload: any }) => {
                const success = event.payload as boolean;
                if (success && isInstalling.current) {
                    setLogs(prev => [...prev, { type: 'success', message: 'uv 安装成功，正在重新启动预览...' }]);
                    // 重新启动预览
                    invoke('retry_preview_after_install', {
                        lang: currentLangRef.current,
                        inputStr: currentInputStrRef.current
                    }).then(() => {
                        isInstalling.current = false;
                    }).catch(error => {
                        setLogs(prev => [...prev, { type: 'error', message: `重新启动预览失败: ${error}` }]);
                        isInstalling.current = false;
                    });
                } else if (!success) {
                    setLogs(prev => [...prev, { type: 'error', message: 'uv 安装失败' }]);
                    isInstalling.current = false;
                }
            };


            try {
                const unlisteners = await Promise.all([
                    listen('artifact-data', handleArtifactData),
                    listen('artifact-log', addLog('log')),
                    listen('artifact-error', addLog('error')),
                    listen('artifact-success', addLog('success')),
                    listen('artifact-redirect', handleRedirect),
                    listen('environment-check', handleEnvironmentCheck),
                    listen('environment-install-started', handleEnvironmentInstallStarted),
                    listen('bun-install-finished', handleBunInstallFinished),
                    listen('uv-install-finished', handleUvInstallFinished)
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
                    await invoke('close_vue_artifact', { previewId: 'vue' });
                } else if (previewTypeRef.current === 'mermaid' || previewTypeRef.current === 'html' || previewTypeRef.current === 'svg' || previewTypeRef.current === 'xml' || previewTypeRef.current === 'markdown' || previewTypeRef.current === 'md') {
                    // Mermaid/HTML/SVG/XML/Markdown 不需要服务器清理，只需要清除DOM
                } else {
                    await invoke('close_react_artifact', { previewId: 'react' });
                }

                setLogs([]);
                setPreviewUrl(null);
                setIsPreviewReady(false);
                setCurrentView('loading');
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
        <div className="flex h-screen bg-background">
            <div className="flex flex-1 flex-col">
                <div className="flex-1 flex flex-col">
                    {currentView === 'loading' ? (
                        /* Loading 视图 - 美观的加载界面 */
                        <div className="flex-1 flex flex-col items-center justify-center p-8 bg-gradient-to-br from-background to-muted/20">
                            {/* Artifact Logo 和标题 */}
                            <div className="flex flex-col items-center mb-8">
                                {/* Logo 容器 */}
                                <div className="relative mb-4">
                                    <div className="w-24 h-24 bg-primary/10 rounded-2xl flex items-center justify-center shadow-lg border border-primary/20">
                                        {/* 根据类型显示不同的图标 */}
                                        {previewType === 'react' ? (
                                            <svg className="w-12 h-12 text-blue-500" fill="currentColor" viewBox="0 0 24 24">
                                                <path d="M12 9.861A2.139 2.139 0 1 0 12 14.139 2.139 2.139 0 1 0 12 9.861zM6.008 16.255l-.472-.12C2.018 15.246 0 13.737 0 11.996s2.018-3.25 5.536-4.139l.472-.119.133.468a23.53 23.53 0 0 0 1.363 3.578l.101.213-.101.213a23.307 23.307 0 0 0-1.363 3.578l-.133.467zM5.317 8.95c-2.674.751-4.315 1.9-4.315 3.046 0 1.145 1.641 2.294 4.315 3.046a24.95 24.95 0 0 1 1.182-3.046A24.752 24.752 0 0 1 5.317 8.95zM17.992 16.255l-.133-.469a23.357 23.357 0 0 0-1.364-3.577l-.101-.213.101-.213a23.42 23.42 0 0 0 1.364-3.578l.133-.468.473.119c3.517.889 5.535 2.398 5.535 4.14s-2.018 3.25-5.535 4.139l-.473.12zm-.491-4.259c.48 1.039.877 2.06 1.182 3.046 2.675-.752 4.315-1.901 4.315-3.046 0-1.146-1.641-2.294-4.315-3.046a24.788 24.788 0 0 1-1.182 3.046zM5.31 8.945l-.133-.467C4.188 4.992 4.488 2.494 6 1.622c1.483-.856 3.864.155 6.359 2.716l.34.349-.34.349a23.552 23.552 0 0 0-2.422 2.967l-.135.193-.235.02a23.657 23.657 0 0 0-3.785.61l-.472.119zm1.896-6.63c-.268 0-.505.058-.705.173-.994.573-1.17 2.565-.485 5.253a25.122 25.122 0 0 1 3.233-.501 24.847 24.847 0 0 1 2.052-2.544c-1.56-1.519-3.037-2.381-4.095-2.381zM16.795 22.677c-.001 0-.001 0 0 0-1.425 0-3.255-1.073-5.154-3.023l-.34-.349.34-.349a23.53 23.53 0 0 0 2.421-2.968l.135-.193.234-.02a23.63 23.63 0 0 0 3.787-.609l.472-.119.134.468c.987 3.484.688 5.983-.824 6.854a2.38 2.38 0 0 1-1.205.308zm-4.096-3.381c1.56 1.519 3.037 2.381 4.095 2.381h.001c.267 0 .505-.058.704-.173.994-.573 1.171-2.566.485-5.254a25.02 25.02 0 0 1-3.234.501 24.674 24.674 0 0 1-2.051 2.545zM18.69 8.945l-.472-.119a23.479 23.479 0 0 0-3.787-.61l-.234-.02-.135-.193a23.414 23.414 0 0 0-2.421-2.967l-.34-.349.34-.349C14.135 1.778 16.515.767 18 1.622c1.512.872 1.812 3.37.823 6.855l-.133.468zM14.75 7.24c1.142.104 2.227.273 3.234.501.686-2.688.509-4.68-.485-5.253-.988-.571-2.845.304-4.8 2.208A24.849 24.849 0 0 1 14.75 7.24zM7.206 22.677A2.38 2.38 0 0 1 6 22.369c-1.512-.871-1.812-3.369-.823-6.854l.132-.468.472.119c1.155.291 2.429.496 3.785.609l.235.02.134.193a23.596 23.596 0 0 0 2.422 2.968l.34.349-.34.349c-1.898 1.95-3.728 3.023-5.151 3.023zm-1.19-6.427c-.686 2.688-.509 4.681.485 5.254.988.571 2.845-.309 4.8-2.208a24.998 24.998 0 0 1-2.052-2.545 24.976 24.976 0 0 1-3.233-.501zM12 16.878c-.823 0-1.669-.036-2.516-.106l-.235-.02-.135-.193a30.388 30.388 0 0 1-1.35-2.122 30.354 30.354 0 0 1-1.166-2.228l-.1-.213.1-.213a30.3 30.3 0 0 1 1.166-2.228c.414-.716.869-1.43 1.35-2.122l.135-.193.235-.02a30.517 30.517 0 0 1 5.033 0l.234.02.134.193a30.672 30.672 0 0 1 2.517 4.35l.101.213-.101.213a30.672 30.672 0 0 1-2.517 4.35l-.134.193-.234.02c-.847.07-1.694.106-2.517.106zm-2.197-1.084c1.48.111 2.914.111 4.395 0a29.006 29.006 0 0 0 2.196-3.798 28.585 28.585 0 0 0-2.197-3.798 29.031 29.031 0 0 0-4.394 0 28.477 28.477 0 0 0-2.197 3.798 29.114 29.114 0 0 0 2.197 3.798z"/>
                                            </svg>
                                        ) : previewType === 'vue' ? (
                                            <svg className="w-12 h-12 text-green-500" fill="currentColor" viewBox="0 0 24 24">
                                                <path d="M2 3h3.5L12 15l6.5-12H22L12 21 2 3zm4.5 0h3L12 7.58 14.5 3h3L12 13.08 6.5 3z"/>
                                            </svg>
                                        ) : previewType === 'html' ? (
                                            <svg className="w-12 h-12 text-orange-500" fill="currentColor" viewBox="0 0 24 24">
                                                <path d="M12 17.56l4.07-1.13.55-6.1H9.38L9.2 8.3h7.6l.2-2.27H6l.6 6.75h6.07l-.23 2.33L12 15.43l-2.44-.68-.16-1.78H7.1l.29 3.28L12 17.56z"/>
                                                <path d="M5 3l2 18 5-1.4 5 1.4 2-18H5z"/>
                                            </svg>
                                        ) : previewType === 'mermaid' ? (
                                            <svg className="w-12 h-12 text-purple-500" fill="currentColor" viewBox="0 0 24 24">
                                                <path d="M21 8v12.15A.85.85 0 0 1 20.15 21H3.85A.85.85 0 0 1 3 20.15V8c0-.47.38-.85.85-.85h16.3c.47 0 .85.38.85.85zM8 9v10h8V9H8zm11-6v3H5V3h14z"/>
                                            </svg>
                                        ) : (
                                            <svg className="w-12 h-12 text-primary" fill="currentColor" viewBox="0 0 24 24">
                                                <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
                                            </svg>
                                        )}
                                    </div>
                                    {/* 加载动画圆环 */}
                                    <div className="absolute -inset-2">
                                        <div className="w-full h-full border-4 border-transparent border-t-primary/30 border-r-primary/30 rounded-full animate-spin"></div>
                                    </div>
                                </div>
                                
                                {/* 标题 */}
                                <h1 className="text-3xl font-bold text-foreground mb-2">
                                    {previewType === 'react' ? 'React Component' :
                                     previewType === 'vue' ? 'Vue Component' :
                                     previewType === 'html' ? 'HTML Content' :
                                     previewType === 'mermaid' ? 'Mermaid Diagram' :
                                     previewType === 'markdown' || previewType === 'md' ? 'Markdown Content' :
                                     'Artifact'}
                                </h1>
                                
                                {/* 副标题 */}
                                <p className="text-lg text-muted-foreground">正在启动预览服务...</p>
                            </div>

                            {/* 状态指示器 */}
                            <div className="flex items-center space-x-2 mb-6">
                                <div className="flex space-x-1">
                                    <div className="w-2 h-2 bg-primary rounded-full animate-pulse"></div>
                                    <div className="w-2 h-2 bg-primary rounded-full animate-pulse" style={{animationDelay: '0.2s'}}></div>
                                    <div className="w-2 h-2 bg-primary rounded-full animate-pulse" style={{animationDelay: '0.4s'}}></div>
                                </div>
                            </div>

                            {/* Log 信息展示区域 */}
                            <div className="w-full max-w-2xl">
                                <div className="bg-card border border-border rounded-lg shadow-sm overflow-hidden">
                                    <div className="px-4 py-3 text-center">
                                        {logs.length === 0 ? (
                                            <div className="text-muted-foreground text-sm py-2">
                                                等待启动...
                                            </div>
                                        ) : (
                                            <div className={`text-sm font-medium transition-all duration-300 ${
                                                logs[logs.length - 1].type === 'error'
                                                    ? 'text-destructive'
                                                    : logs[logs.length - 1].type === 'success'
                                                        ? 'text-green-600 dark:text-green-400'
                                                        : 'text-foreground'
                                            }`}>
                                                {logs[logs.length - 1].message}
                                            </div>
                                        )}
                                    </div>
                                </div>
                            </div>

                            {/* 如果预览准备好了，显示成功状态 */}
                            {isPreviewReady && (previewUrl || previewType === 'mermaid' || previewType === 'html' || previewType === 'svg' || previewType === 'xml' || previewType === 'markdown' || previewType === 'md') && (
                                <div className="mt-6 flex items-center space-x-3 px-4 py-3 bg-green-50 dark:bg-green-950/50 border border-green-200 dark:border-green-800 rounded-lg">
                                    <div className="w-5 h-5 bg-green-500 rounded-full flex items-center justify-center">
                                        <svg className="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 20 20">
                                            <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                                        </svg>
                                    </div>
                                    <p className="text-green-700 dark:text-green-400 font-medium">
                                        预览准备完成，即将自动切换...
                                    </p>
                                </div>
                            )}
                        </div>
                    ) : (
                        /* 预览视图 - 根据类型显示不同内容 */
                        <div className="flex-1 flex flex-col relative">
                            {/* 悬浮刷新按钮 - 仅在支持刷新的类型中显示 */}
                            {previewType !== 'mermaid' && previewType !== 'html' && previewType !== 'svg' && previewType !== 'xml' && previewType !== 'markdown' && previewType !== 'md' && (
                                <button
                                    onClick={handleRefresh}
                                    className="fixed bottom-4 right-4 w-12 h-12 bg-primary hover:bg-primary/90 text-primary-foreground shadow-lg hover:shadow-xl transition-all rounded-full flex items-center justify-center z-50"
                                    title="刷新预览"
                                >
                                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                                    </svg>
                                </button>
                            )}
                            
                            {previewType === 'mermaid' ? (
                                /* Mermaid 图表预览 */
                                <div className="flex-1 flex flex-col p-4">
                                    <div className="flex justify-between items-center mb-2">
                                        <div className="text-sm text-muted-foreground">
                                            缩放: {Math.round(mermaidScale * 100)}% | 提示: 滚轮缩放，空格键+拖动
                                        </div>
                                        <button
                                            onClick={resetMermaidView}
                                            className="px-3 py-1 bg-secondary hover:bg-secondary/80 text-secondary-foreground text-xs rounded transition-colors"
                                        >
                                            重置视图
                                        </button>
                                    </div>
                                    <div
                                        ref={mermaidContainerRef}
                                        className={`flex-1 bg-background border border-border rounded-lg shadow-sm overflow-hidden relative ${
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
                                <div className="flex-1 overflow-auto bg-background p-6">
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
                                    className="flex-1 w-full border-0 bg-background"
                                    sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-presentation"
                                    style={{
                                        minHeight: '400px'
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
            
            {/* 环境安装确认对话框 */}
            <EnvironmentInstallDialog
                tool={environmentTool}
                message={environmentMessage}
                isOpen={showEnvironmentDialog}
                onConfirm={handleEnvironmentInstallConfirm}
                onCancel={handleEnvironmentInstallCancel}
            />
        </div>
    );
} 
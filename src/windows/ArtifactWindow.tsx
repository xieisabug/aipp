import { useEffect, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from '@tauri-apps/api/core';
import { useTheme } from '@/hooks/useTheme';

interface ArtifactCollection {
    id: number;
    name: string;
    icon: string;
    description: string;
    artifact_type: string;
    code: string;
    tags?: string;
    created_time: string;
    last_used_time?: string;
    use_count: number;
}

export default function ArtifactWindow() {
    useTheme();

    const [artifact, setArtifact] = useState<ArtifactCollection | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const loadArtifact = async () => {
            try {
                const window = getCurrentWebviewWindow();
                const windowLabel = window.label;
                
                // 从窗口标签中提取 artifact ID (格式: artifact_123)
                const match = windowLabel.match(/^artifact_(\d+)$/);
                if (!match) {
                    throw new Error(`无效的窗口标签: ${windowLabel}`);
                }

                const artifactId = parseInt(match[1], 10);
                console.log('正在加载 artifact ID:', artifactId);

                // 直接调用后端 API 获取 artifact 数据
                const artifactData = await invoke<ArtifactCollection>('get_artifact_by_id', {
                    id: artifactId
                });

                if (!artifactData) {
                    throw new Error('Artifact 不存在');
                }

                setArtifact(artifactData);
                console.log('Artifact 加载成功:', artifactData);
            } catch (err) {
                console.error('加载 artifact 失败:', err);
                setError(err instanceof Error ? err.message : String(err));
            } finally {
                setIsLoading(false);
            }
        };

        loadArtifact();
    }, []);

    const renderContent = () => {
        if (!artifact) return null;

        switch (artifact.artifact_type) {
            case 'html':
            case 'svg':
            case 'xml':
                // 直接渲染 HTML/SVG/XML 内容，无需外部服务器
                return (
                    <div className="w-full h-full overflow-auto">
                        <div 
                            className="w-full h-full"
                            dangerouslySetInnerHTML={{ __html: artifact.code }}
                        />
                    </div>
                );
            
            case 'markdown':
            case 'md':
                // 对于 Markdown，显示为预格式化文本（可以后续集成 react-markdown）
                return (
                    <div className="w-full h-full overflow-auto p-6">
                        <div className="max-w-4xl mx-auto">
                            <div className="prose prose-sm max-w-none dark:prose-invert bg-muted/50 rounded-lg p-4">
                                <pre className="whitespace-pre-wrap font-mono text-sm">
                                    {artifact.code}
                                </pre>
                            </div>
                        </div>
                    </div>
                );
            
            case 'mermaid':
                // 对于 Mermaid，暂时显示代码（可以后续集成 mermaid 渲染）
                return (
                    <div className="w-full h-full overflow-auto p-6">
                        <div className="max-w-4xl mx-auto">
                            <div className="text-center mb-6">
                                <h2 className="text-lg font-semibold mb-2">Mermaid 图表</h2>
                                <p className="text-muted-foreground text-sm">
                                    完整的图表渲染功能正在开发中，当前显示源代码
                                </p>
                            </div>
                            <div className="bg-muted rounded-lg p-4">
                                <pre className="whitespace-pre-wrap font-mono text-sm">
                                    {artifact.code}
                                </pre>
                            </div>
                        </div>
                    </div>
                );
            
            case 'react':
            case 'vue':
                // 对于 React/Vue，提供两种选项：查看代码或启动预览
                return (
                    <div className="w-full h-full flex flex-col">
                        <div className="flex-shrink-0 p-4 border-b border-border bg-muted/30">
                            <div className="flex items-center justify-between">
                                <div>
                                    <h2 className="font-semibold">
                                        {artifact.artifact_type.toUpperCase()} 组件
                                    </h2>
                                    <p className="text-sm text-muted-foreground">
                                        查看源代码或启动实时预览
                                    </p>
                                </div>
                                <button
                                    className="px-4 py-2 bg-primary text-primary-foreground rounded hover:bg-primary/90 transition-colors"
                                    onClick={async () => {
                                        try {
                                            // 调用现有的预览功能
                                            await invoke('run_artifacts', {
                                                lang: artifact.artifact_type,
                                                inputStr: artifact.code
                                            });
                                        } catch (error) {
                                            console.error('启动预览失败:', error);
                                        }
                                    }}
                                >
                                    启动实时预览
                                </button>
                            </div>
                        </div>
                        <div className="flex-1 overflow-auto p-4">
                            <div className="bg-muted rounded-lg p-4">
                                <pre className="whitespace-pre-wrap font-mono text-sm">
                                    {artifact.code}
                                </pre>
                            </div>
                        </div>
                    </div>
                );
            
            default:
                return (
                    <div className="w-full h-full flex items-center justify-center">
                        <div className="text-center max-w-md">
                            <p className="mb-4 text-muted-foreground">
                                不支持的 artifact 类型: {artifact.artifact_type}
                            </p>
                            <div className="bg-muted rounded-lg p-4 text-left">
                                <pre className="whitespace-pre-wrap font-mono text-sm">
                                    {artifact.code.substring(0, 500)}
                                    {artifact.code.length > 500 ? '...' : ''}
                                </pre>
                            </div>
                        </div>
                    </div>
                );
        }
    };

    if (isLoading) {
        return (
            <div className="flex items-center justify-center h-screen bg-background">
                <div className="text-center">
                    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
                    <p className="text-muted-foreground">加载 Artifact...</p>
                </div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="flex items-center justify-center h-screen bg-background">
                <div className="text-center">
                    <p className="text-destructive mb-2">{error}</p>
                    <p className="text-sm text-muted-foreground">
                        请检查 Artifact 是否存在或重试
                    </p>
                </div>
            </div>
        );
    }

    if (!artifact) {
        return (
            <div className="flex items-center justify-center h-screen bg-background">
                <div className="text-center">
                    <p className="text-muted-foreground mb-2">无法加载 Artifact</p>
                    <p className="text-sm text-muted-foreground">
                        Artifact 可能已被删除
                    </p>
                </div>
            </div>
        );
    }

    return (
        <div className="h-screen bg-background flex flex-col">
            {/* 可选的顶部标题栏 */}
            <div className="flex-shrink-0 px-4 py-2 border-b border-border bg-background/95 backdrop-blur-sm">
                <div className="flex items-center gap-2">
                    <span className="text-lg">{artifact.icon}</span>
                    <h1 className="font-medium">{artifact.name}</h1>
                    <span className="text-xs text-muted-foreground">
                        {artifact.artifact_type.toUpperCase()}
                    </span>
                </div>
            </div>

            {/* 主要内容区域 */}
            <div className="flex-1 overflow-hidden">
                {renderContent()}
            </div>
        </div>
    );
}
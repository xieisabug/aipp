import { useMemo, useCallback } from 'react';
import React from 'react';
import { Components } from 'react-markdown';
import { open } from '@tauri-apps/plugin-shell';
import CodeBlock from '@/components/CodeBlock';
import { 
    REMARK_PLUGINS, 
    REHYPE_PLUGINS, 
    MARKDOWN_COMPONENTS_BASE 
} from '@/constants/markdown';

interface UseMarkdownConfigOptions {
    onCodeRun?: (lang: string, code: string) => void;
    disableMarkdownSyntax?: boolean;
}

interface CustomComponents extends Components {
    antthinking: React.ElementType;
}

export const useMarkdownConfig = ({ onCodeRun, disableMarkdownSyntax = false }: UseMarkdownConfigOptions = {}) => {
    // 换行处理函数 - 完全按原样展示文本，保留所有换行和空行
    const renderTextWithBreaks = useCallback((children: React.ReactNode): React.ReactNode => {
        if (typeof children === 'string') {
            // 使用 white-space: pre-wrap 样式来保留所有空白字符和换行
            return <span style={{ whiteSpace: 'pre-wrap' }}>{children}</span>;
        }
        return children;
    }, []);
    // 使用 useMemo 缓存 markdown 组件配置
    const markdownComponents = useMemo(
        (): CustomComponents => ({
            ...MARKDOWN_COMPONENTS_BASE,
            // 根据 disableMarkdownSyntax 决定如何渲染标准 Markdown 元素
            ...(disableMarkdownSyntax ? {
                // 纯文本模式：重写标准 Markdown 组件为纯文本渲染，支持换行
                h1: ({ children }) => <span>#{' '}{renderTextWithBreaks(children)}</span>,
                h2: ({ children }) => <span>##{' '}{renderTextWithBreaks(children)}</span>,
                h3: ({ children }) => <span>###{' '}{renderTextWithBreaks(children)}</span>,
                h4: ({ children }) => <span>####{' '}{renderTextWithBreaks(children)}</span>,
                h5: ({ children }) => <span>#####{' '}{renderTextWithBreaks(children)}</span>,
                h6: ({ children }) => <span>######{' '}{renderTextWithBreaks(children)}</span>,
                strong: ({ children }) => <span>**{children}**</span>,
                em: ({ children }) => <span>*{children}*</span>,
                blockquote: ({ children }) => <span>{'> '}{renderTextWithBreaks(children)}</span>,
                ul: ({ children }) => <div>{children}</div>,
                ol: ({ children }) => <div>{children}</div>,
                li: ({ children }) => <div>- {renderTextWithBreaks(children)}</div>,
                p: ({ children }) => <div>{renderTextWithBreaks(children)}</div>,
                br: () => <br />,
            } : {}),
            // antthinking自定义组件
            antthinking: ({ children }: { children: any }) => (
                <div>
                    <div
                        className="bg-primary/10 text-primary px-2 py-1 rounded text-sm font-medium inline-block"
                        title={children}
                        data-thinking={children}
                    >
                        思考...
                    </div>
                </div>
            ),
            code: ({ className, children }) => {
                const match = /language-(\w+)/.exec(className || '');
                
                // 纯文本模式：代码块显示为原始文本
                if (disableMarkdownSyntax) {
                    return match ? (
                        <span>```{match[1]}{'\n'}{children}{'\n'}```</span>
                    ) : (
                        <span>`{children}`</span>
                    );
                }
                
                // Markdown 模式：正常的代码块渲染
                return match ? (
                    <CodeBlock
                        language={match[1]}
                        onCodeRun={onCodeRun || (() => {})}
                    >
                        {children}
                    </CodeBlock>
                ) : (
                    <code
                        className={className}
                        style={{ overflow: 'auto' }}
                    >
                        {children}
                    </code>
                );
            },
            a: ({ href, children, ...props }) => {
                const handleClick = useCallback(
                    (e: React.MouseEvent) => {
                        e.preventDefault();
                        if (href) {
                            open(href).catch(console.error);
                        }
                    },
                    [href],
                );

                return (
                    <a
                        href={href}
                        onClick={handleClick}
                        className="text-primary hover:text-primary/80 underline cursor-pointer"
                        {...props}
                    >
                        {children}
                    </a>
                );
            },
        }),
        [onCodeRun, disableMarkdownSyntax, renderTextWithBreaks],
    );

    // 根据 disableMarkdownSyntax 决定使用哪些插件
    const remarkPlugins = useMemo(() => {
        if (disableMarkdownSyntax) {
            // 纯文本模式：只保留自定义组件处理
            return [
                REMARK_PLUGINS.find(plugin => plugin.name === 'remarkCustomCompenent') || REMARK_PLUGINS[3]
            ].filter(Boolean);
        }
        // Markdown 模式：使用所有插件
        return REMARK_PLUGINS;
    }, [disableMarkdownSyntax]);

    const rehypePlugins = useMemo(() => {
        if (disableMarkdownSyntax) {
            // 纯文本模式：简化的 rehype 插件配置
            return [
                REHYPE_PLUGINS[0], // rehypeRaw
                REHYPE_PLUGINS[1], // rehypeSanitize
            ];
        }
        // Markdown 模式：使用所有插件
        return REHYPE_PLUGINS;
    }, [disableMarkdownSyntax]);

    return {
        remarkPlugins,
        rehypePlugins,
        markdownComponents,
    };
};
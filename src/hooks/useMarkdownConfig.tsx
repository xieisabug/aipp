import { useMemo, useCallback } from 'react';
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
}

export const useMarkdownConfig = ({ onCodeRun }: UseMarkdownConfigOptions = {}) => {
    // 使用 useMemo 缓存 markdown 组件配置
    const markdownComponents = useMemo(
        (): Components => ({
            ...MARKDOWN_COMPONENTS_BASE,
            code: ({ className, children }) => {
                const match = /language-(\w+)/.exec(className || '');
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
                        className="text-blue-600 hover:text-blue-800 underline cursor-pointer"
                        {...props}
                    >
                        {children}
                    </a>
                );
            },
        }),
        [onCodeRun],
    );

    return {
        remarkPlugins: REMARK_PLUGINS,
        rehypePlugins: REHYPE_PLUGINS,
        markdownComponents,
    };
};
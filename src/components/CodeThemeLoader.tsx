import React, { useEffect, useState } from 'react';
import { useDisplayConfig } from '../hooks/useDisplayConfig';
import { useTheme } from '../hooks/useTheme';
import { listen } from '@tauri-apps/api/event';

interface CodeThemeLoaderProps {
    children: React.ReactNode;
}

// 导入所有主题CSS文件
import githubLight from '../assets/highlight-themes/github.css?url';
import vs from '../assets/highlight-themes/vs.css?url';
import atomOneLight from '../assets/highlight-themes/atom-one-light.css?url';
import base16Github from '../assets/highlight-themes/base16/github.css?url';
import githubDark from '../assets/highlight-themes/github-dark.css?url';
import githubDarkDimmed from '../assets/highlight-themes/github-dark-dimmed.css?url';
import vs2015 from '../assets/highlight-themes/vs2015.css?url';
import atomOneDark from '../assets/highlight-themes/atom-one-dark.css?url';
import atomOneDarkReasonable from '../assets/highlight-themes/atom-one-dark-reasonable.css?url';

// 主题文件映射
const THEME_FILES: Record<string, string> = {
    // Light themes
    'github': githubLight,
    'vs': vs,
    'atom-one-light': atomOneLight,
    'base16/github': base16Github,
    
    // Dark themes
    'github-dark': githubDark,
    'github-dark-dimmed': githubDarkDimmed,
    'vs2015': vs2015,
    'atom-one-dark': atomOneDark,
    'atom-one-dark-reasonable': atomOneDarkReasonable,
};

const CodeThemeLoader: React.FC<CodeThemeLoaderProps> = ({ children }) => {
    const { config, refreshConfig } = useDisplayConfig();
    const { resolvedTheme } = useTheme();
    const [currentTheme, setCurrentTheme] = useState<string>('github-dark');
    const [loadedTheme, setLoadedTheme] = useState<string>('');
    const [forceUpdate, setForceUpdate] = useState(0);

    // 获取当前应该使用的主题
    const getCurrentTheme = (): string => {
        if (!config) return 'github-dark';
        
        return resolvedTheme === 'dark' 
            ? config.code_theme_dark || 'github-dark'
            : config.code_theme_light || 'github';
    };

    // 加载主题CSS文件
    const loadTheme = async (themeId: string) => {
        if (loadedTheme === themeId) return;

        // 移除之前的主题link
        const existingLinks = document.querySelectorAll('link[data-code-theme]');
        existingLinks.forEach(link => link.remove());

        // 添加新的主题link
        const themeFile = THEME_FILES[themeId];
        if (themeFile) {
            const link = document.createElement('link');
            link.rel = 'stylesheet';
            link.href = themeFile;
            link.setAttribute('data-code-theme', themeId);
            
            // 等待CSS加载完成
            await new Promise((resolve) => {
                link.onload = resolve;
                link.onerror = resolve; // 即使失败也继续
                document.head.appendChild(link);
            });
            
            setLoadedTheme(themeId);
            console.log(`Code theme loaded: ${themeId}`);
        }
    };

    // 监听主题变化
    useEffect(() => {
        const newTheme = getCurrentTheme();
        if (newTheme !== currentTheme || forceUpdate > 0) {
            setCurrentTheme(newTheme);
            loadTheme(newTheme);
        }
    }, [config, resolvedTheme, currentTheme, forceUpdate]);

    // 监听主题变化事件
    useEffect(() => {
        const unlistenThemeChange = listen('theme-changed', async (event) => {
            console.log('Theme change event received:', event.payload);
            
            // 刷新配置
            await refreshConfig();
            
            // 强制更新主题
            setForceUpdate(prev => prev + 1);
        });

        return () => {
            unlistenThemeChange.then(f => f());
        };
    }, [refreshConfig]);

    // 初始加载
    useEffect(() => {
        const initialTheme = getCurrentTheme();
        setCurrentTheme(initialTheme);
        loadTheme(initialTheme);
    }, []);

    return (
        <>
            {children}
        </>
    );
};

export default CodeThemeLoader;
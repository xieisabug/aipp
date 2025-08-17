import { useState, useEffect, useCallback } from 'react';
import { useDisplayConfig } from './useDisplayConfig';
import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';

export type ThemeMode = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

interface ThemeState {
    mode: ThemeMode;
    resolvedTheme: ResolvedTheme;
    systemTheme: ResolvedTheme;
}

export const useTheme = () => {
    const { config, isLoading, refreshConfig } = useDisplayConfig();
    const [themeState, setThemeState] = useState<ThemeState>({
        mode: 'system',
        resolvedTheme: 'light',
        systemTheme: 'light'
    });

    // 检测系统主题
    const detectSystemTheme = useCallback((): ResolvedTheme => {
        if (typeof window !== 'undefined' && window.matchMedia) {
            return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        }
        return 'light';
    }, []);

    // 应用主题到DOM
    const applyTheme = useCallback((theme: ResolvedTheme) => {
        const root = document.documentElement;
        if (theme === 'dark') {
            root.classList.add('dark');
        } else {
            root.classList.remove('dark');
        }
    }, []);

    // 计算最终主题
    const resolveTheme = useCallback((mode: ThemeMode, systemTheme: ResolvedTheme): ResolvedTheme => {
        switch (mode) {
            case 'light':
                return 'light';
            case 'dark':
                return 'dark';
            case 'system':
                return systemTheme;
            default:
                return 'light';
        }
    }, []);

    // 监听系统主题变化
    useEffect(() => {
        if (typeof window === 'undefined' || !window.matchMedia) return;

        const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
        const handleChange = (e: MediaQueryListEvent) => {
            const newSystemTheme: ResolvedTheme = e.matches ? 'dark' : 'light';
            setThemeState(prev => {
                const newResolvedTheme = resolveTheme(prev.mode, newSystemTheme);
                applyTheme(newResolvedTheme);
                return {
                    ...prev,
                    systemTheme: newSystemTheme,
                    resolvedTheme: newResolvedTheme
                };
            });
        };

        // 使用现代API
        if (mediaQuery.addEventListener) {
            mediaQuery.addEventListener('change', handleChange);
            return () => mediaQuery.removeEventListener('change', handleChange);
        } else {
            // 兼容旧版本
            mediaQuery.addListener(handleChange);
            return () => mediaQuery.removeListener(handleChange);
        }
    }, [resolveTheme, applyTheme]);

    // 监听配置变化
    useEffect(() => {
        if (!config || isLoading) return;

        const mode = (config.color_mode as ThemeMode) || 'system';
        const systemTheme = detectSystemTheme();
        const resolvedTheme = resolveTheme(mode, systemTheme);

        setThemeState({
            mode,
            resolvedTheme,
            systemTheme
        });

        // 同步到localStorage以供加载页面使用
        localStorage.setItem('theme-mode', mode);

        applyTheme(resolvedTheme);
    }, [config, isLoading, detectSystemTheme, resolveTheme, applyTheme]);

    // 初始化时设置主题
    useEffect(() => {
        const systemTheme = detectSystemTheme();
        const mode: ThemeMode = 'system'; // 默认值，会被config覆盖
        const resolvedTheme = resolveTheme(mode, systemTheme);
        
        setThemeState({
            mode,
            resolvedTheme,
            systemTheme
        });

        applyTheme(resolvedTheme);
    }, [detectSystemTheme, resolveTheme, applyTheme]);

    // 监听跨窗口主题同步事件
    useEffect(() => {
        const unlistenThemeChange = listen('theme-changed', () => {
            refreshConfig();
        });

        return () => {
            unlistenThemeChange.then(f => f());
        };
    }, [refreshConfig]);

    // 设置主题模式
    const setThemeMode = useCallback(async (newMode: ThemeMode) => {
        try {
            // 保存到后端
            await invoke('save_feature_config', {
                featureCode: 'display',
                config: {
                    theme: config?.theme || 'default',
                    color_mode: newMode,
                    user_message_markdown_render: config?.user_message_markdown_render || 'enabled'
                }
            });

            // 同步到localStorage以供加载页面使用
            localStorage.setItem('theme-mode', newMode);

            // 发出主题变化事件，通知其他窗口
            await emit('theme-changed', { mode: newMode });

            // 刷新配置
            refreshConfig();
        } catch (error) {
            console.error('Failed to save theme mode:', error);
        }
    }, [config, refreshConfig]);

    // 切换主题（在light和dark之间切换）
    const toggleTheme = useCallback(() => {
        const newMode = themeState.resolvedTheme === 'dark' ? 'light' : 'dark';
        setThemeMode(newMode);
    }, [themeState.resolvedTheme, setThemeMode]);

    return {
        mode: themeState.mode,
        resolvedTheme: themeState.resolvedTheme,
        systemTheme: themeState.systemTheme,
        setThemeMode,
        toggleTheme,
        isLoading
    };
};
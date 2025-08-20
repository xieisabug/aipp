import { useCallback } from 'react';
import { useDisplayConfig } from './useDisplayConfig';
import { useTheme } from './useTheme';

// 可用的代码主题配置
export interface CodeThemeOption {
    id: string;
    name: string;
    category: 'light' | 'dark';
}

export const AVAILABLE_CODE_THEMES: CodeThemeOption[] = [
    // 浅色主题
    { id: 'github', name: 'GitHub', category: 'light' },
    { id: 'vs', name: 'Visual Studio', category: 'light' },
    { id: 'atom-one-light', name: 'Atom One Light', category: 'light' },
    { id: 'base16/github', name: 'GitHub Base16', category: 'light' },
    
    // 深色主题
    { id: 'github-dark', name: 'GitHub Dark', category: 'dark' },
    { id: 'github-dark-dimmed', name: 'GitHub Dark Dimmed', category: 'dark' },
    { id: 'vs2015', name: 'Visual Studio 2015', category: 'dark' },
    { id: 'atom-one-dark', name: 'Atom One Dark', category: 'dark' },
    { id: 'atom-one-dark-reasonable', name: 'Atom One Dark Reasonable', category: 'dark' },
];

export const useCodeTheme = () => {
    const { config } = useDisplayConfig();
    const { resolvedTheme } = useTheme();

    // 获取当前应该使用的主题
    const getCurrentTheme = useCallback((): string => {
        if (!config) return 'github-dark';
        
        return resolvedTheme === 'dark' 
            ? config.code_theme_dark || 'github-dark'
            : config.code_theme_light || 'github';
    }, [config, resolvedTheme]);

    // 预设主题选项
    const getLightThemes = useCallback((): CodeThemeOption[] => {
        return AVAILABLE_CODE_THEMES.filter(theme => theme.category === 'light');
    }, []);

    const getDarkThemes = useCallback((): CodeThemeOption[] => {
        return AVAILABLE_CODE_THEMES.filter(theme => theme.category === 'dark');
    }, []);

    return {
        currentTheme: getCurrentTheme(),
        getLightThemes,
        getDarkThemes,
        availableThemes: AVAILABLE_CODE_THEMES
    };
};
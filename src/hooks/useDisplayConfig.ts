import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface DisplayConfig {
    theme: string;
    color_mode: string;
    user_message_markdown_render: string;
}

interface DisplayConfigState {
    config: DisplayConfig | null;
    isLoading: boolean;
    error: string | null;
}

const DEFAULT_CONFIG: DisplayConfig = {
    theme: 'default',
    color_mode: 'system',
    user_message_markdown_render: 'enabled'
};

export const useDisplayConfig = () => {
    const [state, setState] = useState<DisplayConfigState>({
        config: null,
        isLoading: true,
        error: null
    });

    const loadConfig = useCallback(async () => {
        try {
            setState(prev => ({ ...prev, isLoading: true, error: null }));
            
            const featureConfigList = await invoke<Array<{
                id: number;
                feature_code: string;
                key: string;
                value: string;
            }>>('get_all_feature_config');
            
            // 提取显示配置
            const displayConfigMap = new Map<string, string>();
            featureConfigList
                .filter(item => item.feature_code === 'display')
                .forEach(item => {
                    displayConfigMap.set(item.key, item.value);
                });
            
            const config: DisplayConfig = {
                theme: displayConfigMap.get('theme') || DEFAULT_CONFIG.theme,
                color_mode: displayConfigMap.get('color_mode') || DEFAULT_CONFIG.color_mode,
                user_message_markdown_render: displayConfigMap.get('user_message_markdown_render') || DEFAULT_CONFIG.user_message_markdown_render,
            };
            
            setState({
                config,
                isLoading: false,
                error: null
            });
        } catch (error) {
            console.error('Failed to load display config:', error);
            setState({
                config: DEFAULT_CONFIG,
                isLoading: false,
                error: error instanceof Error ? error.message : 'Unknown error'
            });
        }
    }, []);

    useEffect(() => {
        loadConfig();
    }, [loadConfig]);

    const isUserMessageMarkdownEnabled = state.config?.user_message_markdown_render === 'enabled';

    return {
        config: state.config,
        isLoading: state.isLoading,
        error: state.error,
        isUserMessageMarkdownEnabled,
        refreshConfig: loadConfig
    };
};
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

type FeatureConfig = Map<string, Map<string, string>>;

interface FeatureConfigListItem {
    id: number;
    feature_code: string;
    key: string;
    value: string;
}

export const useFeatureConfig = () => {
    const [featureConfig, setFeatureConfig] = useState<FeatureConfig>(new Map());
    const [loading, setLoading] = useState(true);

    // 加载功能配置
    const loadFeatureConfig = useCallback(() => {
        setLoading(true);
        return invoke<Array<FeatureConfigListItem>>("get_all_feature_config")
            .then((feature_config_list) => {
                const newFeatureConfig = new Map<string, Map<string, string>>();
                for (let feature_config of feature_config_list) {
                    let feature_code = feature_config.feature_code;
                    let key = feature_config.key;
                    let value = feature_config.value;
                    if (!newFeatureConfig.has(feature_code)) {
                        newFeatureConfig.set(feature_code, new Map());
                    }
                    newFeatureConfig.get(feature_code)?.set(key, value);
                }
                setFeatureConfig(newFeatureConfig);
                return newFeatureConfig;
            })
            .catch((e) => {
                toast.error("获取配置失败: " + e);
                throw e;
            })
            .finally(() => {
                setLoading(false);
            });
    }, []);

    // 保存功能配置
    const saveFeatureConfig = useCallback((featureCode: string, config: Record<string, any>) => {
        return invoke("save_feature_config", {
            featureCode,
            config,
        }).then(() => {
            // 重新加载配置
            return loadFeatureConfig();
        });
    }, [loadFeatureConfig]);

    // 获取特定功能的配置
    const getConfigByFeature = useCallback((featureCode: string) => {
        return featureConfig.get(featureCode) || new Map();
    }, [featureConfig]);

    // 获取特定功能的特定配置项
    const getConfigValue = useCallback((featureCode: string, key: string, defaultValue: string = "") => {
        return featureConfig.get(featureCode)?.get(key) || defaultValue;
    }, [featureConfig]);

    // 初始化加载
    useEffect(() => {
        loadFeatureConfig();
    }, [loadFeatureConfig]);

    return {
        featureConfig,
        loading,
        loadFeatureConfig,
        saveFeatureConfig,
        getConfigByFeature,
        getConfigValue,
    };
};
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "../../styles/FeatureAssistantConfig.css";
import ConfigForm from "../ConfigForm";
import { toast } from 'sonner';
import { useForm } from "react-hook-form";

interface ModelForSelect {
    name: string;
    code: string;
    id: number;
    llm_provider_id: number;
}

type FeatureConfig = Map<string, Map<string, string>>;

interface FeatureConfigListItem {
    id: number;
    feature_code: string;
    key: string;
    value: string;
}

const FeatureAssistantConfig: React.FC = () => {
    // 基础数据
    // 模型数据
    const [models, setModels] = useState<ModelForSelect[]>([]);
    useEffect(() => {
        invoke<Array<ModelForSelect>>("get_models_for_select").then(
            (modelList) => {
                setModels(modelList);
            }).catch((e) => {
                toast.error('获取模型列表失败: ' + e);
            });
    }, []);

    const [featureConfig, setFeatureConfig] = useState<FeatureConfig>(
        new Map(),
    );
    useEffect(() => {
        invoke<Array<FeatureConfigListItem>>("get_all_feature_config").then(
            (feature_config_list) => {
                for (let feature_config of feature_config_list) {
                    let feature_code = feature_config.feature_code;
                    let key = feature_config.key;
                    let value = feature_config.value;
                    if (!featureConfig.has(feature_code)) {
                        featureConfig.set(feature_code, new Map());
                    }
                    featureConfig.get(feature_code)?.set(key, value);
                }
                console.log("init featureConfig", featureConfig);
                setFeatureConfig(new Map(featureConfig));

                summaryFormReturnData.reset({
                    model: `${featureConfig.get("conversation_summary")?.get("provider_id")}%%${featureConfig.get("conversation_summary")?.get("model_code")}`,
                    summary_length: featureConfig.get("conversation_summary")?.get("summary_length") + "",
                    prompt: featureConfig.get("conversation_summary")?.get("prompt") || "",
                });

                previewFormReturnData.reset({
                    preview_type: featureConfig.get("preview")?.get("preview_type") || "service",
                    nextjs_port: featureConfig.get("preview")?.get("nextjs_port") || "3001",
                    nuxtjs_port: featureConfig.get("preview")?.get("nuxtjs_port") || "3002",
                    auth_token: featureConfig.get("preview")?.get("auth_token") || "",
                });
            },
        ).catch((e) => {
            toast.error('获取配置失败: ' + e);
        });
    }, []);

    const summaryFormReturnData = useForm({
        defaultValues: {
            model: `${featureConfig.get("conversation_summary")?.get("provider_id")}%%${featureConfig.get("conversation_summary")?.get("model_code")}`,
            summary_length: featureConfig.get("conversation_summary")?.get("summary_length") + "",
            prompt: featureConfig.get("conversation_summary")?.get("prompt") || "",
        },
    });

    const handleSaveSummary = useCallback(() => {
        if (!featureConfig.get("conversation_summary")?.has("provider_id")) {
            toast.error("请选择一个模型");
            return;
        }
        if (!featureConfig.get("conversation_summary")?.has("model_code")) {
            toast.error("请选择一个模型");
            return;
        }
        const summaryFormValues = summaryFormReturnData.getValues();
        const [provider_id, model_code] = (summaryFormValues.model as string).split("%%");

        invoke("save_feature_config", {
            featureCode: "conversation_summary",
            config: {
                provider_id,
                model_code,
                summary_length: summaryFormValues.summary_length,
                prompt: summaryFormValues.prompt,
            }
        }).then(() => {
            toast.success('保存成功');
        });
    }, [featureConfig, summaryFormReturnData]);

    const previewFormReturnData = useForm({
        defaultValues: {
            preview_type: featureConfig.get("preview")?.get("preview_type") || "service",
            nextjs_port: featureConfig.get("preview")?.get("nextjs_port") || "3001",
            nuxtjs_port: featureConfig.get("preview")?.get("nuxtjs_port") || "3002",
            auth_token: featureConfig.get("preview")?.get("auth_token") || "",
        },
    });

    const handleSavePreview = useCallback(() => {
        if (!featureConfig.get("preview")?.has("preview_type")) {
            toast.error("请选择一个部署方式");
            return;
        }

        invoke("save_feature_config", {
            featureCode: "preview",
            config: previewFormReturnData.getValues()
        }).then(() => {
            toast.success('保存成功');
        });
    }, [featureConfig, previewFormReturnData]);

    const SUMMARY_FORM_CONFIG = [
        {
            key: "model",
            config: {
                type: "select" as const,
                label: "Model",
                options: models.map((m) => ({
                    value: `${m.llm_provider_id}%%${m.code}`,
                    label: m.name,
                })),
            }
        },
        {
            key: "summary_length",
            config: {
                type: "select" as const,
                label: "总结文本长度",
                options: [50, 100, 300, 500, 1000, -1].map((m) => ({
                    value: m.toString(),
                    label: m === -1 ? "所有" : m.toString(),
                })),
            }
        },
        {
            key: "prompt",
            config: {
                type: "textarea" as const,
                label: "Prompt",
            }
        }
    ];

    const PREVIEW_FORM_CONFIG = [
        {
            key: "preview_type",
            config: {
                type: "radio" as const,
                label: "部署方式",
                options: [
                    { value: "local", label: "本地" },
                    { value: "remote", label: "远程" },
                    { value: "service", label: "使用服务" },
                ],
            }
        },
        {
            key: "nextjs_port",
            config: {
                type: "input" as const,
                label: "Next.js端口",
            }
        },
        {
            key: "nuxtjs_port",
            config: {
                type: "input" as const,
                label: "Nuxt.js端口",
            }
        },
        {
            key: "auth_token",
            config: {
                type: "input" as const,
                label: "Auth token",
            }
        }
    ];

    const handleOpenDataFolder = useCallback(() => {
        invoke("open_data_folder");
    }, []);

    const handleSyncData = useCallback(() => {
        toast.info('暂未实现，敬请期待');
    }, []);

    const DATA_FOLDER_CONFIG = [
        {
            key: "openDataFolder",
            config: {
                type: "button" as const,
                label: "数据文件夹",
                value: "打开",
                onClick: handleOpenDataFolder,
            }
        },
        {
            key: "syncData",
            config: {
                type: "button" as const,
                label: "远程数据",
                value: "同步",
                onClick: handleSyncData,
            }
        }
    ];

    const dataFolderFormReturnData = useForm({});

    return (
        <div className="feature-assistant-editor">
            <ConfigForm
                title="对话总结"
                description="对话开始时总结该对话并且生成标题"
                config={SUMMARY_FORM_CONFIG}
                layout="prompt"
                classNames="bottom-space"
                onSave={handleSaveSummary}
                useFormReturn={summaryFormReturnData}
            />

            <ConfigForm
                title="预览配置"
                description="在大模型编写完react或者vue组件之后，能够快速预览"
                config={PREVIEW_FORM_CONFIG}
                layout="default"
                classNames="bottom-space"
                onSave={handleSavePreview}
                useFormReturn={previewFormReturnData}
            />

            <ConfigForm
                title="数据目录"
                description="管理和同步数据文件夹"
                config={DATA_FOLDER_CONFIG}
                layout="default"
                classNames="bottom-space"
                useFormReturn={dataFolderFormReturnData}
            />
        </div>
    );
};

export default FeatureAssistantConfig;

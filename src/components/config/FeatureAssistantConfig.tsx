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
                setFeatureConfig(new Map(featureConfig));
            },
        ).catch((e) => {
            toast.error('获取配置失败: ' + e);
        });
    }, []);

    // 总结相关表单
    const handleSaveSummary = useCallback(() => {
        const values = summaryFormReturnData.getValues();
        if (!featureConfig.get("conversation_summary")?.has("provider_id")) {
            toast.error("请选择一个模型");
            return;
        }
        if (!featureConfig.get("conversation_summary")?.has("model_code")) {
            toast.error("请选择一个模型");
            return;
        }
        const [provider_id, model_code] = (values.model as string).split("%%");

        invoke("save_feature_config", {
            featureCode: "conversation_summary",
            config: {
                provider_id,
                model_code,
                summary_length: values.summary_length,
                prompt: values.prompt,
            }
        }).then(() => {
            toast.success('保存成功');
        });
    }, []);

    const modelOptions = useMemo(() =>
        models.map((m) => ({
            value: `${m.llm_provider_id}%%${m.code}`,
            label: m.name,
        }))
        , [models]);

    const summaryLengthOptions = useMemo(() =>
        [50, 100, 300, 500, 1000, -1].map((m) => ({
            value: m.toString(),
            label: m === -1 ? "所有" : m.toString(),
        }))
        , []);

    const SUMMARY_FORM_CONFIG = useMemo(() => [
        {
            key: "model",
            config: {
                type: "select" as const,
                label: "Model",
                options: modelOptions,
            }
        },
        {
            key: "summary_length",
            config: {
                type: "select" as const,
                label: "总结文本长度",
                options: summaryLengthOptions,
            }
        },
        {
            key: "prompt",
            config: {
                type: "textarea" as const,
                label: "Prompt",
            }
        }
    ], [modelOptions]);

    const summaryFormReturnData = useForm({
        defaultValues: {
            model: `${featureConfig.get("conversation_summary")?.get("provider_id")}%%${featureConfig.get("conversation_summary")?.get("model_code")}`,
            summary_length: featureConfig.get("conversation_summary")?.get("summary_length") + "",
            prompt: featureConfig.get("conversation_summary")?.get("prompt") || "",
        },
    });

    // 预览相关表单
    const handleSavePreview = useCallback(() => {
        const values = previewFormReturnData.getValues();
        if (!values.preview_type) {
            toast.error("请选择一个部署方式");
            return;
        }

        invoke("save_feature_config", {
            featureCode: "preview",
            config: values
        }).then(() => {
            toast.success('保存成功');
        });
    }, []);

    const PREVIEW_FORM_CONFIG = useMemo(() => [
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
                value: "",
            }
        },
        {
            key: "nuxtjs_port",
            config: {
                type: "input" as const,
                label: "Nuxt.js端口",
                value: "",
            }
        },
        {
            key: "auth_token",
            config: {
                type: "input" as const,
                label: "Auth token",
                value: "",
            }
        }
    ], []);

    const previewFormReturnData = useForm({
        defaultValues: {
            preview_type: featureConfig.get("preview")?.get("preview_type") || "service",
            nextjs_port: featureConfig.get("preview")?.get("nextjs_port") || "3001",
            nuxtjs_port: featureConfig.get("preview")?.get("nuxtjs_port") || "3002",
            auth_token: featureConfig.get("preview")?.get("auth_token") || "",
        },
    });

    // 数据目录相关表单
    const handleOpenDataFolder = useCallback(() => {
        invoke("open_data_folder");
    }, []);

    const handleSyncData = useCallback(() => {
        toast.info('暂未实现，敬请期待');
    }, []);

    const DATA_FOLDER_CONFIG = useMemo(() => [
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
    ], []);

    const dataFolderFormReturnData = useForm({});


    useEffect(() => {
        if (featureConfig.size > 0) {
            summaryFormReturnData.setValue("model", `${featureConfig.get("conversation_summary")?.get("provider_id")}%%${featureConfig.get("conversation_summary")?.get("model_code")}`);
            summaryFormReturnData.setValue("summary_length", featureConfig.get("conversation_summary")?.get("summary_length") || "100");
            summaryFormReturnData.setValue("prompt", featureConfig.get("conversation_summary")?.get("prompt") || "");

            previewFormReturnData.setValue("preview_type", featureConfig.get("preview")?.get("preview_type") || "service");
            previewFormReturnData.setValue("nextjs_port", featureConfig.get("preview")?.get("nextjs_port") || "3001");
            previewFormReturnData.setValue("nuxtjs_port", featureConfig.get("preview")?.get("nuxtjs_port") || "3002");
            previewFormReturnData.setValue("auth_token", featureConfig.get("preview")?.get("auth_token") || "");
        }
    }, [featureConfig, summaryFormReturnData, previewFormReturnData]);

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

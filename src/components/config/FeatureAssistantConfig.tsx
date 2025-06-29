import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../ui/card";
import ConfigForm from "../ConfigForm";
import { MessageSquare, Eye, FolderOpen, Settings, AlertCircle, Zap } from "lucide-react";
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

interface FeatureItem {
    id: string;
    name: string;
    description: string;
    icon: React.ReactNode;
    code: string;
}

const FeatureAssistantConfig: React.FC = () => {
    // 功能列表定义
    const featureList: FeatureItem[] = [
        {
            id: 'conversation_summary',
            name: '对话总结',
            description: '对话开始时总结该对话并且生成标题',
            icon: <MessageSquare className="h-5 w-5" />,
            code: 'conversation_summary'
        },
        {
            id: 'preview',
            name: '预览配置',
            description: '在大模型编写完react或者vue组件之后，能够快速预览',
            icon: <Eye className="h-5 w-5" />,
            code: 'preview'
        },
        {
            id: 'data_folder',
            name: '数据目录',
            description: '管理和同步数据文件夹',
            icon: <FolderOpen className="h-5 w-5" />,
            code: 'data_folder'
        }
    ];

    const [selectedFeature, setSelectedFeature] = useState<FeatureItem>(featureList[0]);

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
            },
        ).catch((e) => {
            toast.error('获取配置失败: ' + e);
        });
    }, []);

    // 选择功能
    const handleSelectFeature = useCallback((feature: FeatureItem) => {
        setSelectedFeature(feature);
    }, []);

    // 统计信息
    const stats = useMemo(() => {
        const total = featureList.length;
        const configured = featureList.filter(feature => {
            const config = featureConfig.get(feature.code);
            return config && config.size > 0;
        }).length;

        return {
            total,
            configured,
            pending: total - configured
        };
    }, [featureConfig, featureList]);

    // 总结相关表单
    const handleSaveSummary = useCallback(() => {
        const values = summaryFormReturnData.getValues();
        if (!values.model || values.model === '-1') {
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
    ], [modelOptions, summaryLengthOptions]);

    const summaryFormReturnData = useForm({
        defaultValues: {
            model: `${featureConfig.get("conversation_summary")?.get("provider_id") || ''}%%${featureConfig.get("conversation_summary")?.get("model_code") || ''}`,
            summary_length: featureConfig.get("conversation_summary")?.get("summary_length") || "100",
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
    ], [handleOpenDataFolder, handleSyncData]);

    const dataFolderFormReturnData = useForm({});

    useEffect(() => {
        if (featureConfig.size > 0) {
            const summaryConfig = featureConfig.get("conversation_summary");
            if (summaryConfig) {
                summaryFormReturnData.setValue("model", `${summaryConfig.get("provider_id") || ''}%%${summaryConfig.get("model_code") || ''}`);
                summaryFormReturnData.setValue("summary_length", summaryConfig.get("summary_length") || "100");
                summaryFormReturnData.setValue("prompt", summaryConfig.get("prompt") || "");
            }

            const previewConfig = featureConfig.get("preview");
            if (previewConfig) {
                previewFormReturnData.setValue("preview_type", previewConfig.get("preview_type") || "service");
                previewFormReturnData.setValue("nextjs_port", previewConfig.get("nextjs_port") || "3001");
                previewFormReturnData.setValue("nuxtjs_port", previewConfig.get("nuxtjs_port") || "3002");
                previewFormReturnData.setValue("auth_token", previewConfig.get("auth_token") || "");
            }
        }
    }, [featureConfig, summaryFormReturnData, previewFormReturnData]);

    // 渲染对应的配置表单
    const renderConfigForm = () => {
        switch (selectedFeature.id) {
            case 'conversation_summary':
                return (
                    <ConfigForm
                        title="对话总结配置"
                        description="配置对话总结的相关参数"
                        config={SUMMARY_FORM_CONFIG}
                        layout="prompt"
                        classNames="bottom-space"
                        onSave={handleSaveSummary}
                        useFormReturn={summaryFormReturnData}
                    />
                );
            case 'preview':
                return (
                    <ConfigForm
                        title="预览配置"
                        description="配置组件预览的相关参数"
                        config={PREVIEW_FORM_CONFIG}
                        layout="default"
                        classNames="bottom-space"
                        onSave={handleSavePreview}
                        useFormReturn={previewFormReturnData}
                    />
                );
            case 'data_folder':
                return (
                    <ConfigForm
                        title="数据目录管理"
                        description="管理和同步数据文件夹"
                        config={DATA_FOLDER_CONFIG}
                        layout="default"
                        classNames="bottom-space"
                        useFormReturn={dataFolderFormReturnData}
                    />
                );
            default:
                return null;
        }
    };

    return (
        <div className="max-w-7xl mx-auto px-4 py-6 space-y-8">


            {/* 主要内容区域 */}
            <div className="grid grid-cols-12 gap-6">
                {/* 左侧功能列表 */}
                <div className="col-span-12 lg:col-span-3">
                    <Card className="bg-gradient-to-br from-gray-50 to-gray-100 border-gray-200 h-fit sticky top-6">
                        <CardHeader className="pb-3">
                            <CardTitle className="text-lg font-semibold text-gray-700 flex items-center gap-2">
                                <Settings className="h-5 w-5" />
                                功能列表
                            </CardTitle>
                            <CardDescription className="text-gray-600">
                                选择功能进行配置
                            </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-3">
                            {featureList.map((feature) => {
                                const config = featureConfig.get(feature.code);
                                const isConfigured = config ? config.size > 0 : false;
                                return (
                                    <Button
                                        key={feature.id}
                                        variant={
                                            selectedFeature.id === feature.id
                                                ? "default"
                                                : "outline"
                                        }
                                        onClick={() => handleSelectFeature(feature)}
                                        className={`
                                            w-full justify-start text-left transition-all duration-200
                                            ${selectedFeature.id === feature.id
                                                ? 'bg-gray-800 hover:bg-gray-900 text-white shadow-md'
                                                : 'hover:bg-gray-50 hover:border-gray-300 text-gray-700'
                                            }
                                        `}
                                    >
                                        <div className="flex items-center w-full">
                                            <div className="flex-1 flex items-center">
                                                {feature.icon}
                                                <div className="ml-3 flex-1 truncate">
                                                    <div className="font-medium truncate">{feature.name}</div>
                                                </div>
                                            </div>
                                            {isConfigured && (
                                                <Zap className="h-3 w-3 ml-2 flex-shrink-0" />
                                            )}
                                        </div>
                                    </Button>
                                );
                            })}
                        </CardContent>
                    </Card>
                </div>

                {/* 右侧配置区域 */}
                <div className="col-span-12 lg:col-span-9">
                    <div className="space-y-6">
                        {/* 功能信息卡片 */}
                        <Card className="bg-white border-gray-200 shadow-sm">
                            <CardHeader className="bg-gradient-to-r from-gray-50 to-gray-100 border-b border-gray-200">
                                <div className="flex items-center justify-between">
                                    <div>
                                        <CardTitle className="text-xl font-bold text-gray-800 flex items-center gap-2">
                                            {selectedFeature.icon}
                                            {selectedFeature.name}
                                            {(() => {
                                                const config = featureConfig.get(selectedFeature.code);
                                                return config && config.size > 0 && (
                                                    <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                                                        已配置
                                                    </span>
                                                );
                                            })()}
                                        </CardTitle>
                                        <CardDescription className="mt-1 text-gray-600">
                                            {selectedFeature.description}
                                        </CardDescription>
                                    </div>
                                </div>
                            </CardHeader>
                        </Card>

                        {/* 配置表单 */}
                        {renderConfigForm()}
                    </div>
                </div>
            </div>
        </div>
    );
};

export default FeatureAssistantConfig;

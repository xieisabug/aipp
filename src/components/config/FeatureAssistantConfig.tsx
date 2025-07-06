import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ConfigForm from "../ConfigForm";
import { MessageSquare, Eye, FolderOpen, Settings } from "lucide-react";
import { toast } from 'sonner';
import { useForm } from "react-hook-form";

// 导入公共组件
import {
    ConfigPageLayout,
    SidebarList,
    ListItemButton,
    SelectOption
} from "../common";

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
                className: "h-64",
                label: "Prompt",
            }
        }
    ], [modelOptions, summaryLengthOptions]);

    const summaryFormReturnData = useForm<{
        model: string;
        summary_length: string;
        prompt: string;
    }>({
        defaultValues: {
            model: `${featureConfig.get("conversation_summary")?.get("provider_id") || ''}%%${featureConfig.get("conversation_summary")?.get("model_code") || ''}`,
            summary_length: featureConfig.get("conversation_summary")?.get("summary_length") || "100",
            prompt: featureConfig.get("conversation_summary")?.get("prompt") || "",
        },
    });

    const [bunVersion, setBunVersion] = useState<string>("");
    const checkBunVersion = useCallback(() => {
        invoke("check_bun_version").then((version) => {
            setBunVersion(version as string);
        });
    }, []);

    const [uvVersion, setUvVersion] = useState<string>("");
    const checkUvVersion = useCallback(() => {
        invoke("check_uv_version").then((version) => {
            setUvVersion(version as string);
        });
    }, []);

    useEffect(() => {
        checkBunVersion();
        checkUvVersion();
    }, [checkBunVersion, checkUvVersion]);

    const PREVIEW_FORM_CONFIG = useMemo(() => [
        bunVersion === "Not Installed" ?
            {
                key: "bun_install",
                config: {
                    type: "button" as const,
                    label: "安装 Bun",
                    value: "安装",
                    onClick: () => {
                        invoke("install_bun");
                    },
                }
            } :
            {
                key: "bun_version",
                config: {
                    type: "static" as const,
                    label: "Bun 版本",
                    value: bunVersion,
                }
            },
        uvVersion === "Not Installed" ?
            {
                key: "uv_install",
                config: {
                    type: "button" as const,
                    label: "安装 UV",
                    value: "安装",
                    onClick: () => {
                        invoke("install_uv");
                    }
                }
            } :
            {
                key: "uv_version",
                config: {
                    type: "static" as const,
                    label: "UV 版本",
                    value: uvVersion,
                }
            }
    ], [bunVersion, uvVersion]);

    const previewFormReturnData = useForm<{
        preview_type: string;
        nextjs_port: string;
        nuxtjs_port: string;
        auth_token: string;
    }>({
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

    // 下拉菜单选项
    const selectOptions: SelectOption[] = useMemo(() =>
        featureList.map(feature => ({
            id: feature.id,
            label: feature.name,
            icon: feature.icon
        })), []);

    // 下拉菜单选择回调
    const handleSelectFromDropdown = useCallback((featureId: string) => {
        const feature = featureList.find(f => f.id === featureId);
        if (feature) {
            handleSelectFeature(feature);
        }
    }, [handleSelectFeature]);

    // 渲染对应的配置表单
    const renderConfigForm = () => {
        switch (selectedFeature.id) {
            case 'conversation_summary':
                return (
                    <ConfigForm
                        title={selectedFeature.name}
                        description={selectedFeature.description}
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
                        title={selectedFeature.name}
                        description={selectedFeature.description}
                        config={PREVIEW_FORM_CONFIG}
                        layout="default"
                        classNames="bottom-space"
                        useFormReturn={previewFormReturnData}
                    />
                );
            case 'data_folder':
                return (
                    <ConfigForm
                        title={selectedFeature.name}
                        description={selectedFeature.description}
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

    // 侧边栏内容
    const sidebar = (
        <SidebarList
            title="功能列表"
            description="选择功能进行配置"
            icon={<Settings className="h-5 w-5" />}
        >
            {featureList.map((feature) => {
                return (
                    <ListItemButton
                        key={feature.id}
                        isSelected={selectedFeature.id === feature.id}
                        onClick={() => handleSelectFeature(feature)}
                    >
                        <div className="flex items-center w-full">
                            <div className="flex-1 flex items-center">
                                {feature.icon}
                                <div className="ml-3 flex-1 truncate">
                                    <div className="font-medium truncate">{feature.name}</div>
                                </div>
                            </div>
                        </div>
                    </ListItemButton>
                );
            })}
        </SidebarList>
    );

    // 右侧内容
    const content = (
        <div className="space-y-6">
            {renderConfigForm()}
        </div>
    );

    return (
        <ConfigPageLayout
            sidebar={sidebar}
            content={content}
            selectOptions={selectOptions}
            selectedOptionId={selectedFeature.id}
            onSelectOption={handleSelectFromDropdown}
            selectPlaceholder="选择功能"
        />
    );
};

export default FeatureAssistantConfig;

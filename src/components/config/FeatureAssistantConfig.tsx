import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import ConfigForm from "../ConfigForm";
import { MessageSquare, Eye, FolderOpen, Settings, Wifi, Monitor } from "lucide-react";
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
            id: 'display',
            name: '显示',
            description: '配置系统外观主题、深浅色模式和用户消息渲染方式',
            icon: <Monitor className="h-5 w-5" />,
            code: 'display'
        },
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
        },
        {
            id: 'network_config',
            name: '网络配置',
            description: '配置请求超时、重试次数和网络代理',
            icon: <Wifi className="h-5 w-5" />,
            code: 'network_config'
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

    // 显示配置相关
    const themeOptions = useMemo(() => [
        { value: 'default', label: '默认主题' }
    ], []);

    const colorModeOptions = useMemo(() => [
        { value: 'light', label: '浅色' },
        { value: 'dark', label: '深色' },
        { value: 'system', label: '跟随系统' }
    ], []);

    const markdownRenderOptions = useMemo(() => [
        { value: 'enabled', label: '开启' },
        { value: 'disabled', label: '关闭' }
    ], []);

    const DISPLAY_FORM_CONFIG = useMemo(() => [
        {
            key: "theme",
            config: {
                type: "select" as const,
                label: "系统外观主题",
                options: themeOptions,
            }
        },
        {
            key: "color_mode",
            config: {
                type: "select" as const,
                label: "深浅色模式",
                options: colorModeOptions,
            }
        },
        {
            key: "user_message_markdown_render",
            config: {
                type: "select" as const,
                label: "用户消息Markdown渲染",
                options: markdownRenderOptions,
            }
        }
    ], [themeOptions, colorModeOptions, markdownRenderOptions]);

    const handleSaveDisplayConfig = useCallback(() => {
        const values = displayFormReturnData.getValues();
        
        invoke("save_feature_config", {
            featureCode: "display",
            config: {
                theme: values.theme,
                color_mode: values.color_mode,
                user_message_markdown_render: values.user_message_markdown_render,
            }
        }).then(() => {
            toast.success('显示配置保存成功');
        }).catch((e) => {
            toast.error('保存显示配置失败: ' + e);
        });
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

    const displayFormReturnData = useForm<{
        theme: string;
        color_mode: string;
        user_message_markdown_render: string;
    }>({
        defaultValues: {
            theme: featureConfig.get("display")?.get("theme") || "default",
            color_mode: featureConfig.get("display")?.get("color_mode") || "system",
            user_message_markdown_render: featureConfig.get("display")?.get("user_message_markdown_render") || "enabled",
        },
    });

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

    // 预览相关表单
    const [bunVersion, setBunVersion] = useState<string>("");
    const [isInstallingBun, setIsInstallingBun] = useState(false);
    const [bunInstallLog, setBunInstallLog] = useState("");

    const checkBunVersion = useCallback(() => {
        invoke("check_bun_version").then((version) => {
            setBunVersion(version as string);
        });
    }, []);

    const [uvVersion, setUvVersion] = useState<string>("");
    const [isInstallingUv, setIsInstallingUv] = useState(false);
    const [uvInstallLog, setUvInstallLog] = useState("");

    const checkUvVersion = useCallback(() => {
        invoke("check_uv_version").then((version) => {
            setUvVersion(version as string);
        });
    }, []);

    useEffect(() => {
        checkBunVersion();
        checkUvVersion();

        const unlistenBunLog = listen('bun-install-log', (event) => {
            setBunInstallLog(prev => prev + "\n" + event.payload);
        });

        const unlistenBunFinished = listen('bun-install-finished', (event) => {
            setTimeout(() => {
                setIsInstallingBun(false);
            }, 1000);
            if (event.payload) {
                toast.success("Bun 安装成功");
                checkBunVersion();
            } else {
                toast.error("Bun 安装失败");
            }
        });

        const unlistenUvLog = listen('uv-install-log', (event) => {
            setUvInstallLog(prev => prev + "\n" + event.payload);
        });

        const unlistenUvFinished = listen('uv-install-finished', (event) => {
            setTimeout(() => {
                setIsInstallingUv(false);
            }, 1000);
            if (event.payload) {
                toast.success("uv 安装成功");
                checkUvVersion();
            } else {
                toast.error("uv 安装失败");
            }
        });

        return () => {
            unlistenBunLog.then(f => f());
            unlistenBunFinished.then(f => f());
            unlistenUvLog.then(f => f());
            unlistenUvFinished.then(f => f());
        };
    }, [checkBunVersion, checkUvVersion]);

    const PREVIEW_FORM_CONFIG = useMemo(() => [
        bunVersion === "Not Installed" ?
            {
                key: "bun_install",
                config: {
                    type: "button" as const,
                    label: "安装 Bun",
                    value: isInstallingBun ? "安装中..." : "安装",
                    onClick: () => {
                        setIsInstallingBun(true);
                        setBunInstallLog("开始进行 Bun 安装...");
                        invoke("install_bun");
                    },
                    disabled: isInstallingBun,
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
        {
            key: "bun_log",
            config: {
                type: "static" as const,
                label: "Bun 安装日志",
                value: bunInstallLog || "",
                hidden: !isInstallingBun,
            }
        },
        uvVersion === "Not Installed" ?
            {
                key: "uv_install",
                config: {
                    type: "button" as const,
                    label: "安装 UV",
                    value: isInstallingUv ? "安装中..." : "安装",
                    onClick: () => {
                        setIsInstallingUv(true);
                        setUvInstallLog("Starting uv installation...");
                        invoke("install_uv");
                    },
                    disabled: isInstallingUv,
                }
            } :
            {
                key: "uv_version",
                config: {
                    type: "static" as const,
                    label: "UV 版本",
                    value: uvVersion,
                }
            },
        {
            key: "uv_log",
            config: {
                type: "static" as const,
                label: "UV 安装日志",
                value: uvInstallLog || "",
                hidden: !isInstallingUv,
            }
        }
    ], [bunVersion, uvVersion, isInstallingBun, isInstallingUv, bunInstallLog, uvInstallLog]);

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

    // 网络配置相关表单
    const handleSaveNetworkConfig = useCallback(() => {
        const values = networkConfigFormReturnData.getValues();
        
        invoke("save_feature_config", {
            featureCode: "network_config",
            config: {
                request_timeout: values.request_timeout,
                retry_attempts: values.retry_attempts,
                network_proxy: values.network_proxy,
            }
        }).then(() => {
            toast.success('网络配置保存成功');
        }).catch((e) => {
            toast.error('保存网络配置失败: ' + e);
        });
    }, []);

    const NETWORK_FORM_CONFIG = useMemo(() => [
        {
            key: "request_timeout",
            config: {
                type: "input" as const,
                label: "请求超时时间（秒）",
                placeholder: "180",
                description: "思考模型返回较慢，不建议设置过低"
            }
        },
        {
            key: "retry_attempts",
            config: {
                type: "input" as const,
                label: "失败重试次数",
                placeholder: "3",
                description: "请求失败时的重试次数"
            }
        },
        {
            key: "network_proxy",
            config: {
                type: "input" as const,
                label: "网络代理",
                placeholder: "http://127.0.0.1:7890",
                description: "支持 http、https 和 socks 协议，例如：http://127.0.0.1:7890"
            }
        }
    ], []);

    const networkConfigFormReturnData = useForm<{
        request_timeout: string;
        retry_attempts: string;
        network_proxy: string;
    }>({
        defaultValues: {
            request_timeout: featureConfig.get("network_config")?.get("request_timeout") || "180",
            retry_attempts: featureConfig.get("network_config")?.get("retry_attempts") || "3", 
            network_proxy: featureConfig.get("network_config")?.get("network_proxy") || "",
        },
    });

    const dataFolderFormReturnData = useForm({});

    useEffect(() => {
        if (featureConfig.size > 0) {
            const displayConfig = featureConfig.get("display");
            if (displayConfig) {
                displayFormReturnData.setValue("theme", displayConfig.get("theme") || "default");
                displayFormReturnData.setValue("color_mode", displayConfig.get("color_mode") || "system");
                displayFormReturnData.setValue("user_message_markdown_render", displayConfig.get("user_message_markdown_render") || "enabled");
            }

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

            const networkConfig = featureConfig.get("network_config");
            if (networkConfig) {
                networkConfigFormReturnData.setValue("request_timeout", networkConfig.get("request_timeout") || "180");
                networkConfigFormReturnData.setValue("retry_attempts", networkConfig.get("retry_attempts") || "3");
                networkConfigFormReturnData.setValue("network_proxy", networkConfig.get("network_proxy") || "");
            }
        }
    }, [featureConfig, displayFormReturnData, summaryFormReturnData, previewFormReturnData, networkConfigFormReturnData]);

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
            case 'display':
                return (
                    <ConfigForm
                        title={selectedFeature.name}
                        description={selectedFeature.description}
                        config={DISPLAY_FORM_CONFIG}
                        layout="default"
                        classNames="bottom-space"
                        useFormReturn={displayFormReturnData}
                        onSave={handleSaveDisplayConfig}
                    />
                );
            case 'conversation_summary':
                return (
                    <ConfigForm
                        title={selectedFeature.name}
                        description={selectedFeature.description}
                        config={SUMMARY_FORM_CONFIG}
                        layout="prompt"
                        classNames="bottom-space"
                        useFormReturn={summaryFormReturnData}
                        onSave={handleSaveSummary}
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
            case 'network_config':
                return (
                    <ConfigForm
                        title={selectedFeature.name}
                        description={selectedFeature.description}
                        config={NETWORK_FORM_CONFIG}
                        layout="default"
                        classNames="bottom-space"
                        useFormReturn={networkConfigFormReturnData}
                        onSave={handleSaveNetworkConfig}
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

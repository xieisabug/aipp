import React, { useCallback, useState, useMemo, useEffect } from "react";
import { MessageSquare, Eye, FolderOpen, Settings, Wifi, Monitor } from "lucide-react";
import { useForm } from "react-hook-form";

// 导入公共组件
import { ConfigPageLayout, SidebarList, ListItemButton, SelectOption } from "../common";

// 导入新的 hooks 和组件
import { useFeatureConfig } from "@/hooks/feature/useFeatureConfig";
import { useVersionManager } from "@/hooks/feature/useVersionManager";
import { FeatureFormRenderer } from "./feature/FeatureFormRenderer";

interface FeatureItem {
    id: string;
    name: string;
    description: string;
    icon: React.ReactNode;
    code: string;
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
            id: "display",
            name: "显示",
            description: "配置系统外观主题、深浅色模式和用户消息渲染方式",
            icon: <Monitor className="h-5 w-5" />,
            code: "display",
        },
        {
            id: "conversation_summary",
            name: "AI总结",
            description: "对话开始时总结该对话并且生成标题，表单自动填写",
            icon: <MessageSquare className="h-5 w-5" />,
            code: "conversation_summary",
        },
        {
            id: "preview",
            name: "预览配置",
            description: "在大模型编写完react或者vue组件之后，能够快速预览",
            icon: <Eye className="h-5 w-5" />,
            code: "preview",
        },
        {
            id: "data_folder",
            name: "数据目录",
            description: "管理和同步数据文件夹",
            icon: <FolderOpen className="h-5 w-5" />,
            code: "data_folder",
        },
        {
            id: "network_config",
            name: "网络配置",
            description: "配置请求超时、重试次数和网络代理",
            icon: <Wifi className="h-5 w-5" />,
            code: "network_config",
        },
    ];

    const [selectedFeature, setSelectedFeature] = useState<FeatureItem>(featureList[0]);

    // 使用新的 hooks
    const { featureConfig, saveFeatureConfig, loading } = useFeatureConfig();
    const versionManager = useVersionManager();

    // 初始化表单
    const displayForm = useForm({
        defaultValues: {
            theme: "default",
            color_mode: "system",
            user_message_markdown_render: "disabled",
            notification_on_completion: "false",
            code_theme_light: "github",
            code_theme_dark: "github-dark",
        },
    });

    const summaryForm = useForm({
        defaultValues: {
            model: "",
            summary_length: "100",
            form_autofill_model: "",
            prompt: "",
        },
    });

    const previewForm = useForm({
        defaultValues: {
            preview_type: "service",
            nextjs_port: "3001",
            nuxtjs_port: "3002",
            auth_token: "",
        },
    });

    const networkForm = useForm({
        defaultValues: {
            request_timeout: "180",
            retry_attempts: "3",
            network_proxy: "",
        },
    });

    const dataFolderForm = useForm({});

    // 监听 featureConfig 变化，更新表单值
    useEffect(() => {
        if (!loading && featureConfig.size > 0) {
            console.log("feature config loaded", featureConfig);
            
            // 更新 display 表单
            const displayConfig = featureConfig.get("display");
            if (displayConfig) {
                displayForm.reset({
                    theme: displayConfig.get("theme") || "default",
                    color_mode: displayConfig.get("color_mode") || "system",
                    user_message_markdown_render: displayConfig.get("user_message_markdown_render") || "disabled",
                    notification_on_completion: displayConfig.get("notification_on_completion") || "false",
                    code_theme_light: displayConfig.get("code_theme_light") || "github",
                    code_theme_dark: displayConfig.get("code_theme_dark") || "github-dark",
                });
            }

            // 更新 summary 表单
            const summaryConfig = featureConfig.get("conversation_summary");
            if (summaryConfig) {
                const providerId = summaryConfig.get("provider_id") || "";
                const modelCode = summaryConfig.get("model_code") || "";
                summaryForm.reset({
                    model: `${providerId}%%${modelCode}`,
                    summary_length: summaryConfig.get("summary_length") || "100",
                    form_autofill_model: summaryConfig.get("form_autofill_model") || "",
                    prompt: summaryConfig.get("prompt") || "",
                });
            }

            // 更新 preview 表单
            const previewConfig = featureConfig.get("preview");
            if (previewConfig) {
                previewForm.reset({
                    preview_type: previewConfig.get("preview_type") || "service",
                    nextjs_port: previewConfig.get("nextjs_port") || "3001",
                    nuxtjs_port: previewConfig.get("nuxtjs_port") || "3002",
                    auth_token: previewConfig.get("auth_token") || "",
                });
            }

            // 更新 network 表单
            const networkConfig = featureConfig.get("network_config");
            if (networkConfig) {
                networkForm.reset({
                    request_timeout: networkConfig.get("request_timeout") || "180",
                    retry_attempts: networkConfig.get("retry_attempts") || "3",
                    network_proxy: networkConfig.get("network_proxy") || "",
                });
            }
        }
    }, [loading, featureConfig, displayForm, summaryForm, previewForm, networkForm]);

    // 选择功能
    const handleSelectFeature = useCallback((feature: FeatureItem) => {
        setSelectedFeature(feature);
    }, []);

    // 保存功能配置的回调函数
    const handleSaveDisplayConfig = useCallback(async () => {
        const values = displayForm.getValues();
        await saveFeatureConfig("display", {
            theme: values.theme,
            color_mode: values.color_mode,
            user_message_markdown_render: values.user_message_markdown_render,
            notification_on_completion: values.notification_on_completion.toString(),
            code_theme_light: values.code_theme_light,
            code_theme_dark: values.code_theme_dark,
        });
    }, [displayForm, saveFeatureConfig]);

    const handleSaveSummaryConfig = useCallback(async () => {
        const values = summaryForm.getValues();
        const [provider_id, model_code] = (values.model as string).split("%%");
        await saveFeatureConfig("conversation_summary", {
            provider_id,
            model_code,
            summary_length: values.summary_length,
            form_autofill_model: values.form_autofill_model,
            prompt: values.prompt,
        });
    }, [summaryForm, saveFeatureConfig]);

    const handleSaveNetworkConfig = useCallback(async () => {
        const values = networkForm.getValues();
        await saveFeatureConfig("network_config", {
            request_timeout: values.request_timeout,
            retry_attempts: values.retry_attempts,
            network_proxy: values.network_proxy,
        });
    }, [networkForm, saveFeatureConfig]);

    // 下拉菜单选项
    const selectOptions: SelectOption[] = useMemo(
        () =>
            featureList.map((feature) => ({
                id: feature.id,
                label: feature.name,
                icon: feature.icon,
            })),
        []
    );

    // 下拉菜单选择回调
    const handleSelectFromDropdown = useCallback(
        (featureId: string) => {
            const feature = featureList.find((f) => f.id === featureId);
            if (feature) {
                handleSelectFeature(feature);
            }
        },
        [handleSelectFeature]
    );

    // 侧边栏内容
    const sidebar = (
        <SidebarList title="功能列表" description="选择功能进行配置" icon={<Settings className="h-5 w-5" />}>
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
            <FeatureFormRenderer
                selectedFeature={selectedFeature}
                forms={{
                    displayForm,
                    summaryForm,
                    previewForm,
                    networkForm,
                    dataFolderForm,
                }}
                versionManager={versionManager}
                onSaveDisplay={handleSaveDisplayConfig}
                onSaveSummary={handleSaveSummaryConfig}
                onSaveNetwork={handleSaveNetworkConfig}
            />
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

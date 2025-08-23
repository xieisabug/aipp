import { useMemo, useCallback } from "react";
import React from "react";
import { AssistantDetail } from "@/data/Assistant";
import { AssistantFormConfig } from "@/types/forms";
import { validateConfig } from "@/utils/validate";
import AssistantMCPFieldDisplay from "@/components/config/AssistantMCPFieldDisplay";

interface UseAssistantFormConfigProps {
    currentAssistant: AssistantDetail | null;
    assistantTypeNameMap: Map<number, string>;
    assistantTypeCustomField: Array<{ key: string; value: Record<string, any> }>;
    assistantTypeCustomLabel: Map<string, string>;
    assistantTypeCustomTips: Map<string, string>;
    assistantTypeHideField: Array<string>;
    navigateTo: (menuKey: string) => void;
    onConfigChange: (key: string, value: string | boolean, value_type: string) => void;
    onPromptChange: (value: string) => void;
}

export const useAssistantFormConfig = ({
    currentAssistant,
    assistantTypeNameMap,
    assistantTypeCustomField,
    assistantTypeCustomLabel,
    assistantTypeCustomTips,
    assistantTypeHideField,
    navigateTo,
    onConfigChange,
    onPromptChange,
}: UseAssistantFormConfigProps) => {
    // 处理配置修改
    const handleConfigChange = useCallback(
        (key: string, value: string | boolean, value_type: string) => {
            if (!currentAssistant) return;

            const { isValid, parsedValue } = validateConfig(value, value_type);
            if (!isValid) return;

            onConfigChange(key, parsedValue, value_type);
        },
        [currentAssistant, onConfigChange]
    );

    // 处理模型变化
    const handleModelChange = useCallback(
        (value: string | boolean) => {
            if (!currentAssistant) return;

            // 这里需要外部组件处理模型变化逻辑
            onConfigChange("model", value as string, "string");
        },
        [currentAssistant, onConfigChange]
    );

    // 生成表单配置
    const formConfig: AssistantFormConfig[] = useMemo(() => {
        if (!currentAssistant) return [];

        console.log((currentAssistant?.model_configs ?? []))

        let baseConfigs: AssistantFormConfig[] = [
            {
                key: "assistantType",
                config: {
                    type: "static" as const,
                    label: assistantTypeCustomLabel.get("assistantType") ?? "助手类型",
                    value: assistantTypeNameMap.get(currentAssistant?.assistant.assistant_type ?? 0) ?? "普通对话助手",
                },
            },
            {
                key: "model",
                config: {
                    type: "model-select" as const,
                    label: assistantTypeCustomLabel.get("model") ?? "Model",
                    value:
                        currentAssistant?.model.length ?? 0 > 0
                            ? `${currentAssistant?.model[0].model_code}%%${currentAssistant?.model[0].provider_id}`
                            : "-1",
                    onChange: handleModelChange,
                },
            },
            ...(currentAssistant?.model_configs ?? [])
                .filter(
                    (config) =>
                        !assistantTypeHideField.includes(config.name) &&
                        !assistantTypeCustomField.find((field) => field.key === config.name)
                )
                .map((config) => ({
                    key: config.name,
                    config: {
                        type: config.value_type === "boolean" ? ("checkbox" as const) : ("input" as const),
                        label: assistantTypeCustomLabel.get(config.name) ?? config.name,
                        value: config.value_type === "boolean" ? config.value == "true" : config.value,
                        tooltip: assistantTypeCustomTips.get(config.name),
                        onChange: (value: string | boolean) =>
                            handleConfigChange(config.name, value, config.value_type),
                        onBlur: (value: string | boolean) =>
                            handleConfigChange(config.name, value as string, config.value_type),
                    },
                })),
            ...assistantTypeCustomField
                .filter((field) => !assistantTypeHideField.includes(field.key))
                .map((field) => ({
                    key: field.key,
                    config: {
                        ...field.value,
                        type: field.value.type,
                        label: assistantTypeCustomLabel.get(field.key) ?? field.value.label,
                        value: (() => {
                            const config = currentAssistant?.model_configs.find((config) => config.name === field.key);
                            if (field.value.type === "checkbox") {
                                return config?.value === "true";
                            } else if (field.value.type === "static") {
                                return config?.value;
                            } else {
                                return config?.value ?? field.value.value ?? "";
                            }
                        })(),
                        tooltip: assistantTypeCustomTips.get(field.key),
                        onChange: (value: string | boolean) =>
                            handleConfigChange(
                                field.key,
                                value,
                                field.value.type === "checkbox" ? "boolean" : "string"
                            ),
                        onBlur: (value: string | boolean) =>
                            handleConfigChange(
                                field.key,
                                value as string,
                                field.value.type === "checkbox" ? "boolean" : "string"
                            ),
                    },
                })),
        ];

        if (!assistantTypeHideField.includes("mcp_config")) {
            baseConfigs.push({
                key: "mcp_config",
                config: {
                    type: "custom" as const,
                    label: "MCP工具",
                    customRender: () => {
                        return React.createElement(AssistantMCPFieldDisplay, {
                            assistantId: currentAssistant?.assistant.id ?? 0,
                            onConfigChange: () => {
                                console.log("MCP configuration changed");
                            },
                            navigateTo: navigateTo,
                        });
                    },
                },
            });
        }

        if (!assistantTypeHideField.includes("prompt")) {
            baseConfigs.push({
                key: "prompt",
                config: {
                    type: "textarea" as const,
                    label: assistantTypeCustomLabel.get("prompt") ?? "Prompt",
                    className: "h-64",
                    value: currentAssistant?.prompts[0]?.prompt ?? "",
                    onChange: (value: string | boolean) => onPromptChange(value as string),
                },
            });
        }

        return baseConfigs;
    }, [
        currentAssistant,
        assistantTypeNameMap,
        assistantTypeCustomField,
        assistantTypeCustomLabel,
        assistantTypeHideField,
        assistantTypeCustomTips,
        handleConfigChange,
        handleModelChange,
        navigateTo,
        onPromptChange,
    ]);

    return { formConfig };
};

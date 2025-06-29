import React, { useEffect, useCallback, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import debounce from "lodash/debounce";
import TagInputContainer from "./TagInputContainer";
import ConfigForm from "../ConfigForm";
import { Switch } from "../ui/switch";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

interface LLMProviderConfigFormProps {
    index: number;
    id: string;
    apiType: string;
    name: string;
    isOffical: boolean;
    enabled: boolean;
    onToggleEnabled: any;
    onDelete: any;
}

const LLMProviderConfigForm: React.FC<LLMProviderConfigFormProps> = ({
    id,
    index,
    apiType,
    name,
    isOffical,
    enabled,
    onDelete,
    onToggleEnabled,
}) => {
    useEffect(() => {
        console.log("Current Props:", {
            id,
            index,
            name,
            apiType,
            isOffical,
            enabled,
            onToggleEnabled: !!onToggleEnabled,
            onDelete: !!onDelete,
        });
    }, [
        id,
        index,
        name,
        apiType,
        isOffical,
        enabled,
        onToggleEnabled,
        onDelete,
    ]);

    const [tags, setTags] = useState<string[]>([]);

    const defaultValues = useMemo(
        () => ({
            endpoint: "",
            api_key: "",
        }),
        [],
    );

    const form = useForm({
        defaultValues,
    });

    // 更新字段
    const updateField = useCallback(
        debounce((key: string, value: string) => {
            invoke("update_llm_provider_config", {
                llmProviderId: id,
                name: key,
                value,
            })
                .then(() => console.log(`Field ${key} updated`))
                .catch((error) =>
                    console.error(`Error updating field ${key}:`, error),
                );
        }, 50),
        [id],
    );

    // 监听字段更新后自动保存
    useEffect(() => {
        // 创建一个订阅
        const subscription = form.watch((value, { name, type }) => {
            if (name && type === "change") {
                // 当有字段变化时，调用对应的保存函数
                updateField(name, value[name] ?? "");
            }
        });

        // 清理订阅
        return () => subscription.unsubscribe();
    }, [form.watch()]);

    // 获取基础数据
    useEffect(() => {
        invoke<Array<LLMProviderConfig>>("get_llm_provider_config", {
            id,
        }).then((configArray) => {
            const newConfig: Record<string, string> = {};
            configArray.forEach((item) => {
                newConfig[item.name] = item.value;
            });
            form.reset(newConfig);
        });

        invoke<Array<LLMModel>>("get_llm_models", {
            llmProviderId: "" + id,
        }).then((modelList) => {
            const newTags = modelList.map((model) => model.name);
            console.log("LLM Provider Config Form", newTags);
            // 调用子组件的方法，更新 tags
            setTags(newTags);
        });
    }, [id]);

    // 获取模型列表
    const fetchModelList = useCallback(async () => {
        invoke<Array<LLMModel>>("fetch_model_list", { llmProviderId: id })
            .then((modelList) => {
                const newTags = modelList.map((model) => model.name);
                // 调用子组件的方法，更新 tags
                setTags(newTags);
                toast.success("获取模型列表成功");
            })
            .catch((e) => {
                toast.error(
                    "获取模型列表失败，请检查Endpoint和Api Key配置: " + e,
                );
            });
    }, [id]);

    const onTagsChange = useCallback((newTags: string[]) => {
        setTags(newTags);
    }, []);
    // 定义稳定的 customRender，不再依赖父组件的状态或函数
    const tagInputRender = useCallback(
        () => (
            <TagInputContainer
                llmProviderId={id}
                tags={tags}
                onTagsChange={onTagsChange}
            />
        ),
        [id, tags],
    );

    // 表单字段定义
    const configFields = useMemo(
        () => [
            {
                key: "apiType",
                config: {
                    type: "static" as const,
                    label: "API类型",
                    value: apiType,
                },
            },
            {
                key: "endpoint",
                config: {
                    type: "input" as const,
                    label: "Endpoint",
                    value: "",
                },
            },
            {
                key: "api_key",
                config: {
                    type: "password" as const,
                    label: "API Key",
                    value: "",
                },
            },
            {
                key: "fetchModelList",
                config: {
                    type: "button" as const,
                    label: "",
                    value: "获取Model列表",
                    onClick: fetchModelList,
                },
            },
            {
                key: "tagInput",
                config: {
                    type: "custom" as const,
                    label: "模型列表",
                    value: "",
                    customRender: tagInputRender,
                },
            },
        ],
        [fetchModelList, tagInputRender],
    );

    const extraButtons = useMemo(
        () => (
            <Switch
                checked={enabled}
                onCheckedChange={() => onToggleEnabled(index)}
            />
        ),
        [enabled, onToggleEnabled, index],
    );

    const onDeleteMemo = useMemo(
        () => (isOffical ? undefined : () => onDelete(id)),
        [isOffical, onDelete, id],
    );

    // 表单部分结束
    return (
        <ConfigForm
            key={id}
            title={name}
            config={configFields}
            classNames="bottom-space"
            onDelete={onDeleteMemo}
            extraButtons={extraButtons}
            useFormReturn={form}
        />
    );
};

export default React.memo(LLMProviderConfigForm, (prevProps, nextProps) => {
    return (
        prevProps.id === nextProps.id &&
        prevProps.index === nextProps.index &&
        prevProps.name === nextProps.name &&
        prevProps.apiType === nextProps.apiType &&
        prevProps.isOffical === nextProps.isOffical &&
        prevProps.enabled === nextProps.enabled &&
        prevProps.onToggleEnabled === nextProps.onToggleEnabled &&
        prevProps.onDelete === nextProps.onDelete
    );
});

import React, { useCallback } from "react";
import { UseFormReturn } from "react-hook-form";
import ConfigForm from "@/components/ConfigForm";
import { ModelForSelect } from "@/hooks/useModels";
import { toast } from "sonner";

interface SummaryConfigFormProps {
    form: UseFormReturn<any>;
    models: ModelForSelect[];
    onSave: () => void;
}

export const SummaryConfigForm: React.FC<SummaryConfigFormProps> = ({ 
    form, 
    models, 
    onSave 
}) => {
    const handleSaveSummary = useCallback(() => {
        const values = form.getValues();
        if (!values.model || values.model === "-1") {
            toast.error("请选择一个模型");
            return;
        }
        onSave();
    }, [form, onSave]);

    const modelOptions = models.map((m) => ({
        value: `${m.llm_provider_id}%%${m.code}`,
        label: m.name,
    }));

    const summaryLengthOptions = [50, 100, 300, 500, 1000, -1].map((m) => ({
        value: m.toString(),
        label: m === -1 ? "所有" : m.toString(),
    }));

    const SUMMARY_FORM_CONFIG = [
        {
            key: "model",
            config: {
                type: "select" as const,
                label: "总结 Model",
                options: modelOptions,
            },
        },
        {
            key: "summary_length",
            config: {
                type: "select" as const,
                label: "总结文本长度",
                options: summaryLengthOptions,
            },
        },
        {
            key: "form_autofill_model",
            config: {
                type: "select" as const,
                label: "表单填写 Model",
                options: modelOptions,
            },
        },
        {
            key: "prompt",
            config: {
                type: "textarea" as const,
                className: "h-64",
                label: "总结 Prompt",
            },
        },
    ];

    return (
        <ConfigForm
            title="AI总结"
            description="对话开始时总结该对话并且生成标题，表单自动填写"
            config={SUMMARY_FORM_CONFIG}
            layout="prompt"
            classNames="bottom-space"
            useFormReturn={form}
            onSave={handleSaveSummary}
        />
    );
};
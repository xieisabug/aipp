import React from "react";
import { UseFormReturn } from "react-hook-form";
import ConfigForm from "@/components/ConfigForm";
import { AssistantDetail } from "@/data/Assistant";
import { AssistantFormConfig, AssistantConfigApi } from "@/types/forms";
import { Button } from "@/components/ui/button";
import { Share } from "lucide-react";

interface AssistantFormRendererProps {
    currentAssistant: AssistantDetail | null;
    formConfig: AssistantFormConfig[];
    form: UseFormReturn<any>;
    assistantConfigApi: AssistantConfigApi;
    onSave: () => void;
    onCopy?: () => void;
    onDelete?: () => void;
    onEdit?: () => void;
    onShare?: () => void;
}

export const AssistantFormRenderer: React.FC<AssistantFormRendererProps> = ({
    currentAssistant,
    formConfig,
    form,
    assistantConfigApi,
    onSave,
    onCopy,
    onDelete,
    onEdit,
    onShare,
}) => {
    if (!currentAssistant) {
        return null;
    }

    return (
        <ConfigForm
            assistantConfigApi={assistantConfigApi}
            title={currentAssistant.assistant.name}
            description={currentAssistant.assistant.description || "配置你的智能助手"}
            config={formConfig}
            layout="prompt"
            classNames="bottom-space"
            onSave={onSave}
            onCopy={currentAssistant.assistant.id === 1 ? undefined : onCopy}
            onDelete={currentAssistant.assistant.id === 1 ? undefined : onDelete}
            onEdit={onEdit}
            useFormReturn={form}
            extraButtons={
                onShare ? (
                    <div className="flex gap-2">
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={onShare}
                            className="gap-2"
                        >
                            <Share className="h-4 w-4" />
                        </Button>
                    </div>
                ) : undefined
            }
        />
    );
};
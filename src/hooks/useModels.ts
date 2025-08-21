import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

export interface ModelForSelect {
    name: string;
    code: string;
    id: number;
    llm_provider_id: number;
}

export const useModels = () => {
    const [models, setModels] = useState<ModelForSelect[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        setLoading(true);
        invoke<Array<ModelForSelect>>("get_models_for_select")
            .then((modelList) => {
                setModels(modelList);
                setError(null);
            })
            .catch((err) => {
                const errorMsg = "获取模型列表失败: " + err;
                setError(errorMsg);
                toast.error(errorMsg);
            })
            .finally(() => {
                setLoading(false);
            });
    }, []);

    return { models, loading, error };
};
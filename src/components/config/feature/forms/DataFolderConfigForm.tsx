import React, { useCallback } from "react";
import { UseFormReturn } from "react-hook-form";
import { invoke } from "@tauri-apps/api/core";
import ConfigForm from "@/components/ConfigForm";
import { toast } from "sonner";

interface DataFolderConfigFormProps {
    form: UseFormReturn<any>;
}

export const DataFolderConfigForm: React.FC<DataFolderConfigFormProps> = ({ form }) => {
    const handleOpenDataFolder = useCallback(() => {
        invoke("open_data_folder");
    }, []);

    const handleSyncData = useCallback(() => {
        toast.info("暂未实现，敬请期待");
    }, []);

    const DATA_FOLDER_CONFIG = [
        {
            key: "openDataFolder",
            config: {
                type: "button" as const,
                label: "数据文件夹",
                value: "打开",
                onClick: handleOpenDataFolder,
            },
        },
        {
            key: "syncData",
            config: {
                type: "button" as const,
                label: "远程数据",
                value: "同步",
                onClick: handleSyncData,
            },
        },
    ];

    return (
        <ConfigForm
            title="数据目录"
            description="管理和同步数据文件夹"
            config={DATA_FOLDER_CONFIG}
            layout="default"
            classNames="bottom-space"
            useFormReturn={form}
        />
    );
};
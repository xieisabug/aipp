import React from "react";
import { UseFormReturn } from "react-hook-form";
import ConfigForm from "@/components/ConfigForm";

interface PreviewConfigFormProps {
    form: UseFormReturn<any>;
    bunVersion: string;
    uvVersion: string;
    isInstallingBun: boolean;
    isInstallingUv: boolean;
    bunInstallLog: string;
    uvInstallLog: string;
    onInstallBun: () => void;
    onInstallUv: () => void;
}

export const PreviewConfigForm: React.FC<PreviewConfigFormProps> = ({
    form,
    bunVersion,
    uvVersion,
    isInstallingBun,
    isInstallingUv,
    bunInstallLog,
    uvInstallLog,
    onInstallBun,
    onInstallUv,
}) => {
    const PREVIEW_FORM_CONFIG = [
        bunVersion === "Not Installed"
            ? {
                  key: "bun_install",
                  config: {
                      type: "button" as const,
                      label: "安装 Bun",
                      value: isInstallingBun ? "安装中..." : "安装",
                      onClick: onInstallBun,
                      disabled: isInstallingBun,
                  },
              }
            : {
                  key: "bun_version",
                  config: {
                      type: "static" as const,
                      label: "Bun 版本",
                      value: bunVersion,
                  },
              },
        {
            key: "bun_log",
            config: {
                type: "static" as const,
                label: "Bun 安装日志",
                value: bunInstallLog || "",
                hidden: !isInstallingBun,
            },
        },
        uvVersion === "Not Installed"
            ? {
                  key: "uv_install",
                  config: {
                      type: "button" as const,
                      label: "安装 UV",
                      value: isInstallingUv ? "安装中..." : "安装",
                      onClick: onInstallUv,
                      disabled: isInstallingUv,
                  },
              }
            : {
                  key: "uv_version",
                  config: {
                      type: "static" as const,
                      label: "UV 版本",
                      value: uvVersion,
                  },
              },
        {
            key: "uv_log",
            config: {
                type: "static" as const,
                label: "UV 安装日志",
                value: uvInstallLog || "",
                hidden: !isInstallingUv,
            },
        },
    ];

    return (
        <ConfigForm
            title="预览配置"
            description="在大模型编写完react或者vue组件之后，能够快速预览"
            config={PREVIEW_FORM_CONFIG}
            layout="default"
            classNames="bottom-space"
            useFormReturn={form}
        />
    );
};
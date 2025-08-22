import React from "react";
import { UseFormReturn } from "react-hook-form";
import { DisplayConfigForm } from "./forms/DisplayConfigForm";
import { SummaryConfigForm } from "./forms/SummaryConfigForm";
import { PreviewConfigForm } from "./forms/PreviewConfigForm";
import { NetworkConfigForm } from "./forms/NetworkConfigForm";
import { DataFolderConfigForm } from "./forms/DataFolderConfigForm";

interface FeatureItem {
    id: string;
    name: string;
    description: string;
    icon: React.ReactNode;
    code: string;
}

interface FeatureFormRendererProps {
    selectedFeature: FeatureItem;
    forms: {
        displayForm: UseFormReturn<any>;
        summaryForm: UseFormReturn<any>;
        previewForm: UseFormReturn<any>;
        networkForm: UseFormReturn<any>;
        dataFolderForm: UseFormReturn<any>;
    };
    versionManager: {
        bunVersion: string;
        uvVersion: string;
        isInstallingBun: boolean;
        isInstallingUv: boolean;
        bunInstallLog: string;
        uvInstallLog: string;
        installBun: () => void;
        installUv: () => void;
    };
    onSaveDisplay: () => Promise<void>;
    onSaveSummary: () => Promise<void>;
    onSaveNetwork: () => Promise<void>;
}

export const FeatureFormRenderer: React.FC<FeatureFormRendererProps> = ({
    selectedFeature,
    forms,
    versionManager,
    onSaveDisplay,
    onSaveSummary,
    onSaveNetwork,
}) => {
    switch (selectedFeature.id) {
        case "display":
            return (
                <DisplayConfigForm
                    form={forms.displayForm}
                    onSave={onSaveDisplay}
                />
            );
        case "conversation_summary":
            return (
                <SummaryConfigForm
                    form={forms.summaryForm}
                    onSave={onSaveSummary}
                />
            );
        case "preview":
            return (
                <PreviewConfigForm
                    form={forms.previewForm}
                    bunVersion={versionManager.bunVersion}
                    uvVersion={versionManager.uvVersion}
                    isInstallingBun={versionManager.isInstallingBun}
                    isInstallingUv={versionManager.isInstallingUv}
                    bunInstallLog={versionManager.bunInstallLog}
                    uvInstallLog={versionManager.uvInstallLog}
                    onInstallBun={versionManager.installBun}
                    onInstallUv={versionManager.installUv}
                />
            );
        case "data_folder":
            return (
                <DataFolderConfigForm
                    form={forms.dataFolderForm}
                />
            );
        case "network_config":
            return (
                <NetworkConfigForm
                    form={forms.networkForm}
                    onSave={onSaveNetwork}
                />
            );
        default:
            return null;
    }
};
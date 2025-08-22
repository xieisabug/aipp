import { PackageOpen, Settings, Store } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import AnimatedLogo from "./AnimatedLogo";
import { useLogoState } from "../hooks/useLogoState";
import { Button } from "./ui/button";

function ChatUIInfomation() {
    const {
        state: logoState,
        showHappy,
        showError,
        showNormal,
    } = useLogoState({
        defaultState: "happy",
        autoReturnToNormal: true,
        autoReturnDelay: 3000,
    });

    const openConfig = async () => {
        try {
            await invoke("open_config_window");
            showHappy();
        } catch (error) {
            showError();
        }
    };

    const openArtifactsCollections = async () => {
        try {
            await invoke("open_artifact_collections_window");
            showHappy();
        } catch (error) {
            showError();
        }
    };

    const openPluginStore = async () => {
        try {
            await invoke("open_plugin_store_window");
            showHappy();
        } catch (error) {
            showError();
        }
    };

    return (
        <div className="flex justify-between py-4 px-5 border-border bg-secondary ">
            <div className="flex items-center gap-2 bg-secondary">
                <AnimatedLogo state={logoState} size={32} onClick={showNormal} />
            </div>
            <div className="flex items-center gap-2">
                <Button onClick={openConfig} variant={"ghost"}>
                    <Settings />
                </Button>
                <Button onClick={openArtifactsCollections} variant={"ghost"}>
                    <PackageOpen />
                </Button>
                <Button onClick={openPluginStore} variant={"ghost"}>
                    <Store />
                </Button>
            </div>
        </div>
    );
}

export default ChatUIInfomation;

import { message } from "@tauri-apps/plugin-dialog";
import { Button } from "./ui/button";

interface ChatUIToolbarProps {
    onNewConversation: () => void;
}

function ChatUIToolbar({ onNewConversation }: ChatUIToolbarProps) {

    const onSearch = async () => {
        message("暂未实现", "很抱歉")
    }

    return (
        <div className="flex flex-none h-12 mb-6 items-center justify-center pt-3">
            <Button className="w-24" onClick={onSearch}>搜索</Button>
            <Button className="w-24 ml-4" onClick={onNewConversation}>新对话</Button>
        </div>
    );
}

export default ChatUIToolbar;

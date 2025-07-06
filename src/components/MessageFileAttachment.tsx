import React from "react";

interface MessageFileAttachmentProps {
    title: string;
    name: string;
}

const MessageFileAttachment: React.FC<MessageFileAttachmentProps> = (props) => {
    const { title, name } = props;

    return (
        <div className="py-3 px-4 bg-slate-50 text-gray-700 border border-gray-200 rounded-lg inline-block cursor-pointer mt-2 text-xs transition-all duration-200 hover:bg-slate-100 hover:border-slate-300 hover:-translate-y-0.5 hover:shadow-lg" title={title}>
            <span>文件名称：{name}</span>
        </div>
    );
};

export default MessageFileAttachment;

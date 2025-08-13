import React from 'react';

interface Attachment {
    attachment_url: string;
    attachment_content: string;
    attachment_type: string;
}

interface ImageAttachmentsProps {
    attachments?: Attachment[];
}

const ImageAttachments: React.FC<ImageAttachmentsProps> = ({ attachments }) => {
    if (!attachments?.length) {
        return null;
    }

    const imageAttachments = attachments.filter(
        (attachment) => attachment.attachment_type === 'Image'
    );

    if (!imageAttachments.length) {
        return null;
    }

    return (
        <div className="w-[300px] flex flex-col">
            {imageAttachments.map((attachment) => (
                <img
                    key={attachment.attachment_url}
                    className="flex-1"
                    src={attachment.attachment_content}
                    alt="Message attachment"
                />
            ))}
        </div>
    );
};

export default ImageAttachments;
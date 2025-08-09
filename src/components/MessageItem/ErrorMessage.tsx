import React from 'react';
import IconButton from '../IconButton';
import Copy from '../../assets/copy.svg?react';
import Ok from '../../assets/ok.svg?react';
import { useCopyHandler } from '@/hooks/useCopyHandler';

interface ErrorMessageProps {
    content: string;
}

const ErrorMessage: React.FC<ErrorMessageProps> = ({ content }) => {
    const { copyIconState, handleCopy } = useCopyHandler(content);

    return (
        <div className="group relative py-4 px-5 rounded-2xl inline-block max-w-[65%] transition-all duration-200 self-start bg-red-50 text-red-800 border border-red-200">
            <div className="flex items-start space-x-3">
                <div className="flex-shrink-0 w-5 h-5 mt-0.5">
                    <svg className="w-5 h-5 text-red-500" fill="currentColor" viewBox="0 0 20 20">
                        <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
                    </svg>
                </div>
                <div className="flex-1">
                    <div className="text-sm font-medium text-red-800 mb-1">
                        AI Request Failed
                    </div>
                    <div className="prose prose-sm max-w-none text-red-700">
                        {content}
                    </div>
                </div>
            </div>
            <div className="hidden group-hover:flex items-center absolute -bottom-9 py-3 px-4 box-border h-10 rounded-[21px] border border-red-200 bg-red-50 left-0">
                <IconButton
                    icon={
                        copyIconState === 'copy' ? (
                            <Copy fill="#dc2626" />
                        ) : (
                            <Ok fill="#dc2626" />
                        )
                    }
                    onClick={handleCopy}
                />
            </div>
        </div>
    );
};

export default ErrorMessage;
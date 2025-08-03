import React from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';

interface VersionPaginationProps {
    currentVersion: number;
    totalVersions: number;
    onVersionChange: (version: number) => void;
    className?: string;
}

const VersionPagination: React.FC<VersionPaginationProps> = ({
    currentVersion,
    totalVersions,
    onVersionChange,
    className = ''
}) => {
    if (totalVersions <= 1) {
        return null;
    }

    const handlePrevious = () => {
        if (currentVersion > 1) {
            onVersionChange(currentVersion - 2); // Convert to 0-based index
        }
    };

    const handleNext = () => {
        if (currentVersion < totalVersions) {
            onVersionChange(currentVersion); // currentVersion is 1-based, so this gives us the next 0-based index
        }
    };

    return (
        <div className={`flex justify-center my-3 ${className}`}>
            <div className="flex items-center bg-gray-50 hover:bg-gray-100 transition-colors duration-200 rounded-lg px-3 py-2 shadow-sm border border-gray-200">
                <button
                    className="flex items-center justify-center w-8 h-8 hover:bg-gray-200 rounded-md disabled:opacity-50 disabled:cursor-not-allowed transition-colors duration-150"
                    disabled={currentVersion <= 1}
                    onClick={handlePrevious}
                    title="上一个版本"
                >
                    <ChevronLeft size={16} className="text-gray-600" />
                </button>
                
                <div className="flex items-center mx-3">
                    <span className="text-sm font-medium text-gray-700">
                        版本 {currentVersion} / {totalVersions}
                    </span>
                </div>
                
                <button
                    className="flex items-center justify-center w-8 h-8 hover:bg-gray-200 rounded-md disabled:opacity-50 disabled:cursor-not-allowed transition-colors duration-150"
                    disabled={currentVersion >= totalVersions}
                    onClick={handleNext}
                    title="下一个版本"
                >
                    <ChevronRight size={16} className="text-gray-600" />
                </button>
            </div>
        </div>
    );
};

export default VersionPagination;
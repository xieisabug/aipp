import { pinyin } from "pinyin-pro";
import { ArtifactCollectionItem, FilteredArtifact } from "../data/ArtifactCollection";

export interface AssistantItem {
    id: number;
    name: string;
    description?: string;
}

export interface FilteredAssistant extends AssistantItem {
    matchType: "exact" | "pinyin" | "initial";
    highlightIndices: number[];
}

/**
 * 将中文字符转换为拼音用于过滤
 * 支持全拼和首字母匹配
 */
export class PinyinFilter {
    /**
     * 基于搜索查询使用名称和拼音过滤工件
     * @param artifacts 要过滤的工件列表
     * @param query 搜索查询（可以是中文字符、拼音或首字母）
     * @returns 过滤并排序的工件列表
     */
    static filterArtifacts(
        artifacts: ArtifactCollectionItem[],
        query: string,
    ): FilteredArtifact[] {
        if (!query.trim()) {
            return artifacts.map((artifact) => ({
                ...artifact,
                matchType: "exact",
                highlightIndices: [],
            }));
        }

        const queryLower = query.toLowerCase();
        const results: FilteredArtifact[] = [];

        for (const artifact of artifacts) {
            const nameLower = artifact.name.toLowerCase();
            const descriptionLower = artifact.description?.toLowerCase() || '';

            // 检查精确名称匹配
            if (nameLower.includes(queryLower)) {
                const indices = this.getMatchIndices(nameLower, queryLower);
                results.push({
                    ...artifact,
                    matchType: "exact",
                    highlightIndices: indices,
                });
                continue;
            }

            // 检查描述匹配
            if (descriptionLower.includes(queryLower)) {
                results.push({
                    ...artifact,
                    matchType: "exact",
                    highlightIndices: [],
                });
                continue;
            }

            // 检查拼音匹配
            try {
                const pinyinArray = pinyin(artifact.name, {
                    toneType: "none",
                    type: "array",
                }).map((p) => p.toLowerCase());

                const pinyinFull = pinyinArray.join("");
                const pinyinWithSpace = pinyinArray.join(" ");
                const pinyinInitials = pinyinArray
                    .map((p) => p.charAt(0))
                    .join("");

                // 全拼匹配（连续拼音，如 "shitu" 匹配 "识图"）
                if (pinyinFull.includes(queryLower)) {
                    const indices = this.getPinyinMatchIndices(
                        pinyinArray,
                        pinyinFull,
                        queryLower,
                    );
                    results.push({
                        ...artifact,
                        matchType: "pinyin",
                        highlightIndices: indices,
                    });
                    continue;
                }

                // 分词拼音匹配（如 "shi tu" 匹配 "识图"）
                if (pinyinWithSpace.includes(queryLower)) {
                    const indices = this.getSpacedPinyinMatchIndices(
                        artifact.name,
                        pinyinWithSpace,
                        queryLower,
                    );
                    results.push({
                        ...artifact,
                        matchType: "pinyin",
                        highlightIndices: indices,
                    });
                    continue;
                }

                // 首字母匹配（如 "stcs" 匹配 "识图测试"）
                if (this.isInitialsMatch(pinyinInitials, queryLower)) {
                    const indices = this.getInitialsMatchIndices(
                        pinyinInitials,
                        queryLower,
                    );
                    results.push({
                        ...artifact,
                        matchType: "initial",
                        highlightIndices: indices,
                    });
                }
            } catch (error) {
                // 降级处理：如果拼音转换失败，跳过拼音匹配
                console.warn("Failed to convert to pinyin:", error);
            }
        }

        // 按匹配类型优先级排序，然后按使用频率和名称排序
        return results.sort((a, b) => {
            // 优先级：精确匹配 > 拼音匹配 > 首字母匹配 > 模糊匹配
            const priority = { exact: 0, pinyin: 1, initial: 2, fuzzy: 3 };
            if (a.matchType !== b.matchType) {
                return priority[a.matchType] - priority[b.matchType];
            }
            // 相同匹配类型下，按使用频率排序（高频率在前）
            if (a.use_count !== b.use_count) {
                return b.use_count - a.use_count;
            }
            return a.name.localeCompare(b.name);
        });
    }

    /**
     * 基于搜索查询使用名称和拼音过滤助手
     * @param assistants 要过滤的助手列表
     * @param query 搜索查询（可以是中文字符、拼音或首字母）
     * @returns 过滤并排序的助手列表
     */
    static filterAssistants(
        assistants: AssistantItem[],
        query: string,
    ): FilteredAssistant[] {
        if (!query.trim()) {
            return assistants.map((assistant) => ({
                ...assistant,
                matchType: "exact",
                highlightIndices: [],
            }));
        }

        const queryLower = query.toLowerCase();
        const results: FilteredAssistant[] = [];

        for (const assistant of assistants) {
            const nameLower = assistant.name.toLowerCase();

            // 检查精确名称匹配
            if (nameLower.includes(queryLower)) {
                const indices = this.getMatchIndices(nameLower, queryLower);
                results.push({
                    ...assistant,
                    matchType: "exact",
                    highlightIndices: indices,
                });
                continue;
            }

            // 检查拼音匹配
            try {
                const pinyinArray = pinyin(assistant.name, {
                    toneType: "none",
                    type: "array",
                }).map((p) => p.toLowerCase());

                const pinyinFull = pinyinArray.join("");
                const pinyinWithSpace = pinyinArray.join(" ");
                const pinyinInitials = pinyinArray
                    .map((p) => p.charAt(0))
                    .join("");

                // 全拼匹配（连续拼音，如 "shitu" 匹配 "识图"）
                if (pinyinFull.includes(queryLower)) {
                    const indices = this.getPinyinMatchIndices(
                        pinyinArray,
                        pinyinFull,
                        queryLower,
                    );
                    results.push({
                        ...assistant,
                        matchType: "pinyin",
                        highlightIndices: indices,
                    });
                    continue;
                }

                // 分词拼音匹配（如 "shi tu" 匹配 "识图"）
                if (pinyinWithSpace.includes(queryLower)) {
                    const indices = this.getSpacedPinyinMatchIndices(
                        assistant.name,
                        pinyinWithSpace,
                        queryLower,
                    );
                    results.push({
                        ...assistant,
                        matchType: "pinyin",
                        highlightIndices: indices,
                    });
                    continue;
                }

                // 首字母匹配（如 "stcs" 匹配 "识图测试"）
                if (this.isInitialsMatch(pinyinInitials, queryLower)) {
                    const indices = this.getInitialsMatchIndices(
                        pinyinInitials,
                        queryLower,
                    );
                    results.push({
                        ...assistant,
                        matchType: "initial",
                        highlightIndices: indices,
                    });
                }
            } catch (error) {
                // 降级处理：如果拼音转换失败，跳过拼音匹配
                console.warn("Failed to convert to pinyin:", error);
            }
        }

        // 按匹配类型优先级排序，然后按名称排序
        return results.sort((a, b) => {
            // 优先级：精确匹配 > 拼音匹配 > 首字母匹配
            const priority = { exact: 0, pinyin: 1, initial: 2 };
            if (a.matchType !== b.matchType) {
                return priority[a.matchType] - priority[b.matchType];
            }
            return a.name.localeCompare(b.name);
        });
    }

    /**
     * 获取匹配字符的索引位置用于高亮显示
     */
    private static getMatchIndices(text: string, query: string): number[] {
        const indices: number[] = [];
        let searchIndex = 0;

        for (let i = 0; i < query.length; i++) {
            const charIndex = text.indexOf(query[i], searchIndex);
            if (charIndex !== -1) {
                indices.push(charIndex);
                searchIndex = charIndex + 1;
            }
        }

        return indices;
    }

    /**
     * 获取全拼匹配的原始字符索引位置
     */
    private static getPinyinMatchIndices(
        pinyinArray: string[],
        pinyinFull: string,
        queryLower: string,
    ): number[] {
        const matchStartIndex = pinyinFull.indexOf(queryLower);
        if (matchStartIndex === -1) return [];

        const matchEndIndex = matchStartIndex + queryLower.length;
        
        // 计算匹配的拼音对应的原始字符位置
        let currentPinyinPos = 0;
        let startCharIndex = -1;
        let endCharIndex = -1;
        
        for (let i = 0; i < pinyinArray.length; i++) {
            const pinyinLen = pinyinArray[i].length;
            
            if (startCharIndex === -1 && currentPinyinPos <= matchStartIndex && currentPinyinPos + pinyinLen > matchStartIndex) {
                startCharIndex = i;
            }
            
            if (currentPinyinPos < matchEndIndex && currentPinyinPos + pinyinLen >= matchEndIndex) {
                endCharIndex = i;
                break;
            }
            
            currentPinyinPos += pinyinLen;
        }
        
        // 返回匹配的字符索引范围
        const indices: number[] = [];
        if (startCharIndex !== -1 && endCharIndex !== -1) {
            for (let i = startCharIndex; i <= endCharIndex; i++) {
                indices.push(i);
            }
        }
        
        return indices;
    }

    /**
     * 获取空格分隔拼音匹配的原始字符索引位置
     */
    private static getSpacedPinyinMatchIndices(
        originalName: string,
        pinyinWithSpace: string,
        queryLower: string,
    ): number[] {
        const matchStartIndex = pinyinWithSpace.indexOf(queryLower);
        if (matchStartIndex === -1) return [];

        // 分析查询在空格分隔字符串中的位置
        const beforeMatch = pinyinWithSpace.substring(0, matchStartIndex);
        const spacesBefore = (beforeMatch.match(/ /g) || []).length;
        
        // 找到起始字符索引
        let startCharIndex = spacesBefore;
        
        // 如果查询不是从完整拼音单词开始，需要调整
        const pinyinWords = pinyinWithSpace.split(' ');
        let remainingQuery = queryLower;
        let currentWordIndex = spacesBefore;
        let matchedCharCount = 0;
        
        while (remainingQuery.length > 0 && currentWordIndex < pinyinWords.length) {
            const currentWord = pinyinWords[currentWordIndex];
            
            if (remainingQuery.startsWith(currentWord)) {
                matchedCharCount++;
                remainingQuery = remainingQuery.substring(currentWord.length);
                if (remainingQuery.startsWith(' ')) {
                    remainingQuery = remainingQuery.substring(1);
                }
                currentWordIndex++;
            } else if (remainingQuery.startsWith(currentWord.substring(0, remainingQuery.length))) {
                // 部分匹配最后一个单词
                matchedCharCount++;
                break;
            } else {
                break;
            }
        }
        
        const indices: number[] = [];
        for (let i = startCharIndex; i < startCharIndex + matchedCharCount && i < originalName.length; i++) {
            indices.push(i);
        }
        
        return indices;
    }

    /**
     * 检查首字母是否匹配（支持非连续匹配）
     * 例如："stcs" 可以匹配 "shituceshi" 的首字母 "stcs"
     */
    private static isInitialsMatch(initials: string, query: string): boolean {
        if (query.length > initials.length) {
            return false;
        }

        let queryIndex = 0;
        for (let i = 0; i < initials.length && queryIndex < query.length; i++) {
            if (initials[i] === query[queryIndex]) {
                queryIndex++;
            }
        }

        return queryIndex === query.length;
    }

    /**
     * 获取首字母匹配的索引位置
     */
    private static getInitialsMatchIndices(
        initials: string,
        query: string,
    ): number[] {
        const indices: number[] = [];
        let queryIndex = 0;

        for (let i = 0; i < initials.length && queryIndex < query.length; i++) {
            if (initials[i] === query[queryIndex]) {
                indices.push(i);
                queryIndex++;
            }
        }

        return indices;
    }

    /**
     * 获取名称的拼音表示用于显示
     */
    static getPinyinDisplay(name: string): string {
        try {
            return pinyin(name, { toneType: "symbol" });
        } catch (error) {
            return name;
        }
    }

    /**
     * 获取首字母用于快速参考
     */
    static getInitials(name: string): string {
        try {
            return pinyin(name, { pattern: "first", toneType: "none" });
        } catch (error) {
            return name.substring(0, 1).toUpperCase();
        }
    }
}

export default PinyinFilter;

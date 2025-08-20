export interface ArtifactCollectionItem {
    id: number;
    name: string;
    icon: string;
    description: string;
    artifact_type: string;
    tags?: string;
    created_time: string;
    last_used_time?: string;
    use_count: number;
}

export interface ArtifactCollection extends ArtifactCollectionItem {
    code: string;
}

export interface SaveArtifactRequest {
    name: string;
    icon: string;
    description: string;
    artifact_type: string;
    code: string;
    tags?: string;
}

export interface UpdateArtifactRequest {
    id: number;
    name?: string;
    icon?: string;
    description?: string;
    tags?: string;
}

export interface FilteredArtifact extends ArtifactCollectionItem {
    matchType: 'exact' | 'pinyin' | 'initial' | 'fuzzy';
    highlightIndices: number[];
}

export interface ArtifactMetadata {
    name: string;
    description: string;
    tags: string;
    emoji_category: string;
}
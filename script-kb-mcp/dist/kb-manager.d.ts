/**
 * Knowledge Base management utilities
 */
import { KBFile, KBDirectory, SearchResult, KBCategory } from './types.js';
export declare class KBManager {
    private readonly kbPath;
    constructor(projectRoot: string);
    /**
     * Validate and normalize a path within the KB directory
     */
    private validatePath;
    /**
     * Check if a path exists
     */
    exists(filePath: string): Promise<boolean>;
    /**
     * Read a file from the KB
     */
    readFile(filePath: string): Promise<KBFile>;
    /**
     * Write or update a file in the KB
     */
    writeFile(filePath: string, content: string): Promise<void>;
    /**
     * Delete a file from the KB
     */
    deleteFile(filePath: string): Promise<void>;
    /**
     * List files in a directory
     */
    listDirectory(dirPath?: string): Promise<KBDirectory>;
    /**
     * Search for content in KB files
     */
    search(query: string, directory?: string): Promise<SearchResult[]>;
    /**
     * Get a list of valid KB categories
     */
    getCategories(): readonly KBCategory[];
    /**
     * Validate if a path belongs to a valid category
     */
    isValidCategory(filePath: string): boolean;
}
//# sourceMappingURL=kb-manager.d.ts.map
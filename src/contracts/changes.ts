export interface FileChange {
  path: string;
  kind: "added" | "modified" | "deleted";
  added: number;
  removed: number;
  beforeSize: number | null;
  afterSize: number | null;
  textAvailable: boolean;
}
export interface FileDiff {
  change: FileChange;
  before: string | null;
  after: string | null;
  lines: { kind: "equal" | "insert" | "delete"; text: string }[];
}

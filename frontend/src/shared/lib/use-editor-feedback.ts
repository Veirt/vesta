import { useState } from "react";
import { uploadIcon } from "@/shared/api/client";

export type LoadState<TReady = { baseline: string }> =
  | { kind: "idle" }
  | { kind: "loading" }
  | ({ kind: "ready" } & TReady)
  | { kind: "error"; message: string };

export type ActionState = { kind: "idle" } | { kind: "saving" } | { kind: "uploading" };

export type FeedbackBanner =
  | { kind: "none" }
  | { kind: "error"; message: string }
  | { kind: "notice"; message: string };

export function useEditorFeedback() {
  const [loadRevision, setLoadRevision] = useState(0);
  const [action, setAction] = useState<ActionState>({ kind: "idle" });
  const [banner, setBanner] = useState<FeedbackBanner>({ kind: "none" });

  function clearNotice() {
    setBanner((current) => (current.kind === "notice" ? { kind: "none" } : current));
  }

  function retryLoad() {
    setLoadRevision((revision) => revision + 1);
  }

  async function uploadImage(file: File, successNote: string): Promise<string | null> {
    setAction({ kind: "uploading" });
    setBanner({ kind: "none" });
    try {
      const uploaded = await uploadIcon(file);
      setBanner({
        kind: "notice",
        message: `${uploaded.name} uploaded. ${successNote}`,
      });
      return uploaded.url;
    } catch (cause) {
      setBanner({
        kind: "error",
        message: cause instanceof Error ? cause.message : "The image could not be uploaded.",
      });
      return null;
    } finally {
      setAction({ kind: "idle" });
    }
  }

  return {
    action,
    banner,
    loadRevision,
    error: banner.kind === "error" ? banner.message : "",
    notice: banner.kind === "notice" ? banner.message : "",
    saving: action.kind === "saving",
    uploading: action.kind === "uploading",
    setAction,
    setBanner,
    clearNotice,
    retryLoad,
    uploadImage,
  };
}

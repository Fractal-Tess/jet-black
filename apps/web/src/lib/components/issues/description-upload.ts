import { api } from "@workspace/convex/api";
import { useMutation } from "convex-svelte";
import type { Id } from "../../../../../../convex/convex/_generated/dataModel";

/**
 * Returns an upload callback for description editors. Must be called during
 * component initialisation (convex-svelte relies on Svelte context).
 */
export function useDescriptionUploader(getWorkspaceId: () => Id<"workspaces">) {
  const generateUploadUrl = useMutation(api.mutations.assets.generateUploadUrl);
  const storeDescriptionAsset = useMutation(
    api.mutations.assets.storeDescriptionAsset
  );

  return async (file: File): Promise<string> => {
    const workspaceId = getWorkspaceId();
    const uploadUrl = await generateUploadUrl({ workspaceId });
    const response = await fetch(uploadUrl, {
      body: file,
      headers: { "Content-Type": file.type },
      method: "POST",
    });

    if (!response.ok) {
      throw new Error("Upload failed");
    }

    const { storageId } = await response.json();

    return await storeDescriptionAsset({ storageId, workspaceId });
  };
}

import * as React from "react";
import { useTranslation } from "react-i18next";
import { FileDropArea } from "./source-section";
import { Button } from "@/components/ui/button";
import { Sparkles, Loader2, CheckCircle2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "@/stores/settings-store";
import { toast } from "sonner";

export function ReferenceVideoSection() {
  const { t } = useTranslation();
  const referenceVideoPath = useSettingsStore((s) => s.referenceVideoPath);
  const updateSetting = useSettingsStore((s) => s.updateSetting);

  const [isAnalyzing, setIsAnalyzing] = React.useState(false);
  const [analyzedSuccess, setAnalyzedSuccess] = React.useState(false);

  const handleFileChange = (filePath: string | null) => {
    updateSetting("referenceVideoPath", filePath);
    setAnalyzedSuccess(false);
  };

  const handleAnalyzeStyle = async () => {
    if (!referenceVideoPath) return;

    setIsAnalyzing(true);
    setAnalyzedSuccess(false);

    try {
      // Call backend command to analyze video styles
      const result = await invoke<any>("analyze_reference_video", {
        videoPath: referenceVideoPath,
      });

      console.log("Extracted style result:", result);
      setAnalyzedSuccess(true);
      toast.success(t("referenceVideo.analysisSuccess", "Styles successfully extracted from reference video!"));
    } catch (error) {
      console.error("Failed to analyze reference video:", error);
      toast.error(t("referenceVideo.analysisError", "Failed to extract styles from video"));
    } finally {
      setIsAnalyzing(false);
    }
  };

  return (
    <div className="flex flex-col gap-2.5">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
          {t("referenceVideo.title", "Reference Video (Style Transfer)")}
        </span>
      </div>

      <div className="flex flex-col gap-2">
        <FileDropArea
          selectedFile={referenceVideoPath}
          onSelectedFileChange={handleFileChange}
          className="min-h-[100px] py-2"
        />

        {referenceVideoPath && (
          <div className="flex items-center gap-2">
            <Button
              onClick={handleAnalyzeStyle}
              disabled={isAnalyzing}
              variant="secondary"
              className="w-full gap-2"
            >
              {isAnalyzing ? (
                <>
                  <Loader2 className="size-4 animate-spin" />
                  {t("referenceVideo.analyzing", "Extracting styles & colors...")}
                </>
              ) : analyzedSuccess ? (
                <>
                  <CheckCircle2 className="size-4 text-green-500" />
                  {t("referenceVideo.reanalyze", "Re-analyze Reference Video")}
                </>
              ) : (
                <>
                  <Sparkles className="size-4 text-primary" />
                  {t("referenceVideo.analyzeButton", "Extract Style & Colors")}
                </>
              )}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}

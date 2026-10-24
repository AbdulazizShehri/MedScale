import Link from "next/link";
import { MODEL_CATALOG_STATS } from "@/lib/catalog-stats";

/**
 * The pinned third-party model discovery index is metadata, not MedScale model
 * releases or installed model weights. Counts verified against the catalog.
 */
export function ModelCatalogScale() {
  const stats = MODEL_CATALOG_STATS;
  return (
    <div className="catalog-scale" aria-label="MedScale external model discovery catalog">
      <div className="catalog-scale-values">
        <div className="catalog-scale-item">
          <strong>{stats.modelRecords.toLocaleString("en-US")}</strong>
          <span>Models indexed</span>
        </div>
        <div className="catalog-scale-item">
          <strong>{stats.languageTags}</strong>
          <span>Languages indexed</span>
        </div>
        <div className="catalog-scale-item">
          <strong>{stats.modelFamilies}</strong>
          <span>Model families</span>
        </div>
      </div>
      <div className="catalog-scale-context">
        External model catalog references · not bundled model weights · <Link href="/product/">Explore MedScale</Link>
      </div>
    </div>
  );
}

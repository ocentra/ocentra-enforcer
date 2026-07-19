import { BookOpenCheck, CircleAlert, Filter, LockKeyhole, Search, SlidersHorizontal, ToggleLeft, ToggleRight, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { Project } from "../data/enforcerAppData";
import {
  type CatalogRule,
  type ProjectRuleCoverage,
  type RuleOverride,
  type RuleSeverity,
  projectRuleLanguages,
  ruleFamilySummary,
  ruleSource,
  rulesForProject,
  unsupportedProjectRuleLanguages,
} from "../data/ruleCatalog";

type CatalogView = "universal" | "detected" | "overrides" | "all";

const viewLabels: Array<{ id: CatalogView; label: string }> = [
  { id: "universal", label: "Universal" },
  { id: "detected", label: "Project stack" },
  { id: "overrides", label: "Overrides" },
  { id: "all", label: "All rules" },
];

export function RuleCatalogWorkspace({
  project,
  catalog,
  catalogLoading,
  catalogError,
  coverage,
  overrides,
  focusRuleId,
  onUpdateOverride,
}: {
  project: Project;
  catalog: CatalogRule[];
  catalogLoading: boolean;
  catalogError: string;
  coverage?: ProjectRuleCoverage | undefined;
  overrides: RuleOverride[];
  focusRuleId?: string | undefined;
  onUpdateOverride: (override: RuleOverride) => Promise<void>;
}) {
  const [view, setView] = useState<CatalogView>("detected");
  const [family, setFamily] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [selectedRuleId, setSelectedRuleId] = useState<string | null>(null);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<CatalogRule | null>(null);

  const scopedRules = useMemo(() => rulesForProject(project, view, overrides, catalog, coverage), [project, view, overrides, catalog, coverage]);
  const policyLanguages = useMemo(() => coverage?.catalogLanguages.filter((language) => language === "common" || coverage.detectedLanguages.includes(language)) ?? projectRuleLanguages(project, catalog), [project, catalog, coverage]);
  const observedWithoutPolicy = useMemo(() => coverage?.observedWithoutCatalog ?? unsupportedProjectRuleLanguages(project, catalog), [project, catalog, coverage]);
  const families = useMemo(() => ruleFamilySummary(scopedRules), [scopedRules]);
  const visibleRules = useMemo(() => {
    const term = query.trim().toLowerCase();
    return scopedRules.filter((rule) => {
      const familyMatches = family === null || `${rule.language}:${rule.family}` === family;
      const queryMatches = !term || `${rule.id} ${rule.title} ${rule.snippet} ${rule.family}`.toLowerCase().includes(term);
      return familyMatches && queryMatches;
    });
  }, [family, query, scopedRules]);

  useEffect(() => {
    if (!visibleRules.some((rule) => rule.id === selectedRuleId)) {
      setSelectedRuleId(visibleRules[0]?.id ?? null);
    }
  }, [selectedRuleId, visibleRules]);

  useEffect(() => {
    if (!focusRuleId) return;
    setView("all");
    setFamily(null);
    setQuery(focusRuleId);
    setSelectedRuleId(focusRuleId);
    setInspectorOpen(true);
  }, [focusRuleId]);

  const selectedRule = visibleRules.find((rule) => rule.id === selectedRuleId) ?? scopedRules.find((rule) => rule.id === selectedRuleId) ?? null;

  return (
    <section className="main-surface rule-catalog-layout">
      <aside className="rule-facet-panel">
        <div className="panel-head">
          <span>
            <strong>Rule scope</strong>
            <small>Policy: {policyLanguages.join(" / ")}</small>
          </span>
          <Filter size={17} />
        </div>
        <div className="rule-view-tabs" role="tablist" aria-label="Rule catalog scope">
          {viewLabels.map((item) => (
            <button key={item.id} className={view === item.id ? "rule-view-tab active" : "rule-view-tab"} onClick={() => { setView(item.id); setFamily(null); }}>
              {item.label}
            </button>
          ))}
        </div>
        {observedWithoutPolicy.length > 0 && <div className="policy-callout rule-coverage-boundary"><CircleAlert size={16} /><span><strong>Observed outside policy registry</strong><small>{observedWithoutPolicy.join(", ")}</small><small>These languages are visible from project inspection, but have no named rule family in the current catalog.</small></span></div>}
        <div className="facet-title"><span>Families</span><em>{scopedRules.length}</em></div>
        <div className="rule-family-stack">
          <button className={family === null ? "rule-family-filter active" : "rule-family-filter"} onClick={() => setFamily(null)}>
            <span>All visible rules</span><em>{scopedRules.length}</em>
          </button>
          {families.map((item) => (
            <button className={family === item.id ? "rule-family-filter active" : "rule-family-filter"} key={item.id} onClick={() => setFamily(item.id)}>
              <span><strong>{item.family}</strong><small>{item.language} / {item.blocking} blocking</small></span>
              <em>{item.count}</em>
            </button>
          ))}
        </div>
      </aside>

      <div className="rule-list-panel">
        <div className="panel-head rule-list-head">
          <span>
            <strong>Rule catalog</strong>
            <small>{visibleRules.length} of {scopedRules.length} rules in the current scope. Numbered rules remain visible even when a profile changes classification.</small>
          </span>
          <label className="rule-search"><Search size={16} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Find ID, title, family" /></label>
        </div>
        <div className="rule-table">
          <div className="catalog-row catalog-header">
            <span>Rule</span><span>Effective</span><span>Source</span><span>State</span>
          </div>
          <div className="catalog-scroll">
            {catalogLoading && <div className="rule-empty">Loading the Rust-owned rule catalog.</div>}
            {!catalogLoading && catalogError && <div className="rule-empty">{catalogError}</div>}
            {!catalogLoading && !catalogError && visibleRules.map((rule) => (
              <button className={rule.id === selectedRuleId ? "catalog-row selected" : "catalog-row"} key={rule.id} onClick={() => { setSelectedRuleId(rule.id); setInspectorOpen(true); }}>
                <span><strong>{rule.id} {rule.title}</strong><small>{rule.snippet}</small></span>
                <em className={`severity-text ${rule.effectiveSeverity}`}>{rule.effectiveSeverity}</em>
                <em title={ruleSource(rule)}>{ruleSource(rule)}</em>
                <i className={rule.override ? "override-state changed" : "override-state"} title={rule.override ? rule.override.enabled ? "overridden" : "waived" : rule.lockLevel}>{rule.override ? rule.override.enabled ? "overridden" : "waived" : rule.lockLevel}</i>
              </button>
            ))}
            {!catalogLoading && !catalogError && visibleRules.length === 0 && <div className="rule-empty">No rules match this project scope and filter.</div>}
          </div>
        </div>
      </div>

      <aside className={inspectorOpen ? "rule-inspector is-open" : "rule-inspector"}>
        {selectedRule ? <RuleInspector rule={selectedRule} onEdit={() => setEditingRule(selectedRule)} onClose={() => setInspectorOpen(false)} /> : <EmptyInspector />}
      </aside>

      {editingRule && (
        <OverrideDialog
          rule={editingRule}
          existing={overrides.find((item) => item.ruleId === editingRule.id)}
          onClose={() => setEditingRule(null)}
          onSave={async (override) => { await onUpdateOverride(override); setEditingRule(null); }}
        />
      )}
    </section>
  );
}

function RuleInspector({ rule, onEdit, onClose }: { rule: ReturnType<typeof rulesForProject>[number]; onEdit: () => void; onClose: () => void }) {
  const fixture = rule.requiresFailFixture && rule.requiresPassFixture ? "pass + fail required" : rule.requiresPassFixture ? "pass fixture" : rule.requiresFailFixture ? "fail fixture" : "no fixture contract";
  return (
    <>
      <div className="detail-heading"><BookOpenCheck size={20} /><span><strong>{rule.id}</strong><small>{rule.family} / {rule.language}</small></span><button className="icon-button inspector-close" onClick={onClose} title="Close rule details"><X size={17} /></button></div>
      <h2>{rule.title}</h2>
      <p>{rule.snippet}</p>
      <dl className="meta-grid rule-meta-grid">
        <dt>Why</dt><dd>{ruleSource(rule)}{rule.lockLevel === "immutable" ? "; this rule cannot be disabled by project policy." : "; project policy may change its effective classification."}</dd>
        <dt>Validator</dt><dd>{rule.validator}</dd>
        <dt>Applies to</dt><dd>{rule.appliesTo.join(", ")}</dd>
        <dt>Path match</dt><dd>{pathMatchSummary(rule.coverage)}</dd>
        <dt>Triggers</dt><dd>{rule.triggers.join(", ")}</dd>
        <dt>Fixtures</dt><dd>{fixture}</dd>
        <dt>Source</dt><dd>{rule.doc}</dd>
      </dl>
      <div className="rule-inspector-status">
        <span className={`severity ${rule.effectiveSeverity}`}>{rule.effectiveSeverity}</span>
        <span>{rule.coverage?.state ?? (rule.override ? rule.override.enabled ? "project override active" : `waiver: ${rule.override.waiver?.owner ?? "owner required"}` : "registry default")}</span>
      </div>
      <button className="primary-action full-width" onClick={onEdit} disabled={!rule.canDisable && !rule.canDowngrade}>
        {rule.canDisable || rule.canDowngrade ? <SlidersHorizontal size={16} /> : <LockKeyhole size={16} />}
        {rule.canDisable || rule.canDowngrade ? "Stage project policy change" : "Locked by registry"}
      </button>
      {rule.override && <div className="policy-callout"><CircleAlert size={17} /><span>{rule.override.enabled ? "This project changes the effective severity." : `Waived by ${rule.override.waiver?.owner}: ${rule.override.waiver?.reason}`}</span></div>}
    </>
  );
}

function pathMatchSummary(coverage: ReturnType<typeof rulesForProject>[number]["coverage"]) {
  if (!coverage) return "Coverage is unavailable for this project.";
  if (coverage.pathMatchStatus === "matched") return `Matches ${coverage.matchedPathCount} project path${coverage.matchedPathCount === 1 ? "" : "s"}.`;
  if (coverage.pathMatchStatus === "no-match") return "No project paths match the declared scope.";
  if (coverage.pathMatchStatus === "invalid-pattern") return "The catalog contains an invalid path pattern; applicability is not asserted.";
  return "This rule declares no path scope.";
}

function EmptyInspector() {
  return <div className="empty-inspector"><BookOpenCheck size={24} /><strong>Select a numbered rule</strong><small>Its source, trigger, fixture contract, and project policy state appear here.</small></div>;
}

function OverrideDialog({ rule, existing, onClose, onSave }: { rule: CatalogRule; existing?: RuleOverride | undefined; onClose: () => void; onSave: (override: RuleOverride) => Promise<void> }) {
  const [enabled, setEnabled] = useState(existing?.enabled ?? true);
  const [severity, setSeverity] = useState<RuleSeverity>(existing?.severity ?? rule.severity);
  const [owner, setOwner] = useState(existing?.waiver?.owner ?? "");
  const [reason, setReason] = useState(existing?.waiver?.reason ?? "");
  const requiresWaiver = !enabled;
  const canSave = !requiresWaiver || (owner.trim().length > 0 && reason.trim().length > 0);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");

  return (
    <div className="policy-dialog-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="policy-dialog" role="dialog" aria-modal="true" aria-label={`Edit ${rule.id} project policy`} onMouseDown={(event) => event.stopPropagation()}>
        <div className="panel-head"><span><strong>Edit project policy</strong><small>{rule.id} / {rule.title}</small></span><button className="icon-button" onClick={onClose} title="Close policy editor"><X size={18} /></button></div>
        <label className="policy-field"><span>Effective state</span><select value={enabled ? "enabled" : "disabled"} onChange={(event) => setEnabled(event.target.value === "enabled")}><option value="enabled">Enabled</option><option value="disabled" disabled={!rule.canDisable}>Disabled with waiver</option></select></label>
        <label className="policy-field"><span>Effective severity</span><select value={severity} disabled={!rule.canDowngrade} onChange={(event) => setSeverity(event.target.value as RuleSeverity)}><option value="error">error</option><option value="warning">warning</option><option value="info">info</option></select></label>
        {requiresWaiver && <div className="waiver-fields"><div className="policy-callout"><CircleAlert size={17} /><span>Disabling a rule requires a named owner and a reason. This mirrors the typed Rust request contract.</span></div><label className="policy-field"><span>Waiver owner</span><input value={owner} onChange={(event) => setOwner(event.target.value)} placeholder="team or person" /></label><label className="policy-field"><span>Waiver reason</span><textarea value={reason} onChange={(event) => setReason(event.target.value)} placeholder="Why this project needs the exception" /></label></div>}
        {saveError && <div className="index-error">{saveError}</div>}
        <div className="dialog-actions"><button className="secondary-action" onClick={onClose} disabled={saving}>Cancel</button><button className="primary-action" disabled={!canSave || saving} onClick={() => { setSaving(true); setSaveError(""); void onSave({ ruleId: rule.id, enabled, severity, waiver: enabled ? undefined : { owner: owner.trim(), reason: reason.trim() } }).catch((error: unknown) => { setSaveError(String(error)); setSaving(false); }); }}>{enabled ? <ToggleRight size={16} /> : <ToggleLeft size={16} />}{saving ? "Saving" : enabled ? "Save override" : "Save waiver"}</button></div>
      </section>
    </div>
  );
}

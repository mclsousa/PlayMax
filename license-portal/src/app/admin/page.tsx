"use client";

import { Fragment, useCallback, useEffect, useState } from "react";

interface LicenseRow {
  id: string;
  license_key: string;
  status: string;
  expires_at: string | null;
  max_devices: number;
  notes: string | null;
  license_activations?: { count: number }[];
}

export default function AdminPage() {
  const [adminKey, setAdminKey] = useState("");
  const [licenses, setLicenses] = useState<LicenseRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editExpires, setEditExpires] = useState("");
  const [editNoExpiry, setEditNoExpiry] = useState(false);
  const [editMaxDevices, setEditMaxDevices] = useState(1);
  const [editNotes, setEditNotes] = useState("");

  const load = useCallback(async () => {
    if (!adminKey.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const response = await fetch("/api/admin/licenses", {
        headers: { "x-admin-key": adminKey.trim() },
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(data.error ?? "Falha ao carregar");
      }
      setLicenses(data.licenses ?? []);
      window.localStorage.setItem("playmax_admin_key", adminKey.trim());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [adminKey]);

  useEffect(() => {
    const saved = window.localStorage.getItem("playmax_admin_key");
    if (saved) setAdminKey(saved);
  }, []);

  async function createLicense() {
    setLoading(true);
    setError(null);
    setCreatedKey(null);
    try {
      const response = await fetch("/api/admin/licenses", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-admin-key": adminKey.trim(),
        },
        body: JSON.stringify({ status: "active" }),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(data.error ?? "Falha ao criar");
      }
      setCreatedKey(data.license.license_key);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  async function patchLicense(body: Record<string, unknown>) {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch("/api/admin/licenses", {
        method: "PATCH",
        headers: {
          "Content-Type": "application/json",
          "x-admin-key": adminKey.trim(),
        },
        body: JSON.stringify(body),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(data.error ?? "Falha na operação");
      }
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  function startEdit(license: LicenseRow) {
    setEditingId(license.id);
    setEditExpires(license.expires_at ? license.expires_at.slice(0, 10) : "");
    setEditNoExpiry(!license.expires_at);
    setEditMaxDevices(license.max_devices ?? 1);
    setEditNotes(license.notes ?? "");
  }

  async function saveEdit(licenseId: string) {
    const expiresAt = editNoExpiry
      ? null
      : editExpires
        ? new Date(`${editExpires}T23:59:59`).toISOString()
        : null;
    await patchLicense({
      licenseId,
      action: "update",
      expiresAt,
      maxDevices: editMaxDevices,
      notes: editNotes.trim() === "" ? null : editNotes.trim(),
    });
    setEditingId(null);
  }

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", maxWidth: 1080, margin: "0 auto", padding: 32 }}>
      <h1>Painel admin — Licenças</h1>
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <input
          type="password"
          placeholder="ADMIN_API_KEY"
          value={adminKey}
          onChange={(event) => setAdminKey(event.target.value)}
          style={{ flex: 1, padding: 8 }}
        />
        <button type="button" onClick={() => void load()} disabled={loading}>
          Entrar
        </button>
        <button type="button" onClick={() => void createLicense()} disabled={loading || !adminKey}>
          Nova licença
        </button>
      </div>

      {createdKey ? (
        <p style={{ background: "#ecfdf5", padding: 12, borderRadius: 8 }}>
          Licença criada: <strong>{createdKey}</strong>
        </p>
      ) : null}
      {error ? <p style={{ color: "#b91c1c" }}>{error}</p> : null}

      <table style={{ width: "100%", borderCollapse: "collapse" }}>
        <thead>
          <tr>
            <th align="left">Chave</th>
            <th align="left">Status</th>
            <th align="left">Vencimento</th>
            <th align="left">PCs</th>
            <th align="left">Nota</th>
            <th align="left">Ações</th>
          </tr>
        </thead>
        <tbody>
          {licenses.map((license) => {
            const activations = license.license_activations?.[0]?.count ?? 0;
            return (
              <Fragment key={license.id}>
                <tr style={{ borderTop: "1px solid #e5e7eb" }}>
                  <td style={{ padding: "8px 4px", fontFamily: "monospace" }}>{license.license_key}</td>
                  <td style={{ padding: "8px 4px" }}>{license.status}</td>
                  <td style={{ padding: "8px 4px" }}>
                    {license.expires_at
                      ? new Date(license.expires_at).toLocaleDateString("pt-BR")
                      : "—"}
                  </td>
                  <td style={{ padding: "8px 4px" }}>
                    {activations} / {license.max_devices ?? 1}
                  </td>
                  <td style={{ padding: "8px 4px", maxWidth: 180, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {license.notes ?? "—"}
                  </td>
                  <td style={{ padding: "8px 4px", display: "flex", gap: 6 }}>
                    {license.status === "revoked" ? (
                      <button
                        type="button"
                        disabled={loading}
                        onClick={() => void patchLicense({ licenseId: license.id, action: "unrevoke" })}
                      >
                        Desbloquear
                      </button>
                    ) : (
                      <button
                        type="button"
                        disabled={loading}
                        onClick={() => void patchLicense({ licenseId: license.id, action: "revoke" })}
                      >
                        Bloquear
                      </button>
                    )}
                    <button
                      type="button"
                      disabled={loading || activations === 0}
                      onClick={() => {
                        if (window.confirm("Resetar a ativação? O PC atual será desconectado.")) {
                          void patchLicense({ licenseId: license.id, action: "reset_devices" });
                        }
                      }}
                    >
                      Resetar PC
                    </button>
                    <button type="button" disabled={loading} onClick={() => startEdit(license)}>
                      Editar
                    </button>
                  </td>
                </tr>
                {editingId === license.id ? (
                  <tr style={{ background: "#f9fafb" }}>
                    <td colSpan={6} style={{ padding: 12 }}>
                      <div style={{ display: "flex", gap: 16, alignItems: "center", flexWrap: "wrap" }}>
                        <label>
                          Vencimento:{" "}
                          <input
                            type="date"
                            value={editExpires}
                            disabled={editNoExpiry}
                            onChange={(event) => setEditExpires(event.target.value)}
                          />
                        </label>
                        <label>
                          <input
                            type="checkbox"
                            checked={editNoExpiry}
                            onChange={(event) => setEditNoExpiry(event.target.checked)}
                          />{" "}
                          Sem vencimento
                        </label>
                        <label>
                          PCs:{" "}
                          <input
                            type="number"
                            min={1}
                            max={10}
                            value={editMaxDevices}
                            onChange={(event) => setEditMaxDevices(Number(event.target.value))}
                            style={{ width: 56 }}
                          />
                        </label>
                        <label>
                          Nota:{" "}
                          <input
                            type="text"
                            value={editNotes}
                            placeholder="nome / WhatsApp do cliente"
                            onChange={(event) => setEditNotes(event.target.value)}
                            style={{ width: 240 }}
                          />
                        </label>
                        <button type="button" disabled={loading} onClick={() => void saveEdit(license.id)}>
                          Salvar
                        </button>
                        <button type="button" onClick={() => setEditingId(null)}>
                          Cancelar
                        </button>
                      </div>
                    </td>
                  </tr>
                ) : null}
              </Fragment>
            );
          })}
        </tbody>
      </table>
    </main>
  );
}

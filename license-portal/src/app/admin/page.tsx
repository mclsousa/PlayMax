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
  const [authed, setAuthed] = useState(false);
  const [licenses, setLicenses] = useState<LicenseRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editExpires, setEditExpires] = useState("");
  const [editNoExpiry, setEditNoExpiry] = useState(false);
  const [editMaxDevices, setEditMaxDevices] = useState(1);
  const [editNotes, setEditNotes] = useState("");

  const load = useCallback(
    async (keyOverride?: string) => {
      const key = (keyOverride ?? adminKey).trim();
      if (!key) return;
      setLoading(true);
      setError(null);
      try {
        const response = await fetch("/api/admin/licenses", {
          headers: { "x-admin-key": key },
        });
        const data = await response.json();
        if (!response.ok) {
          throw new Error(data.error ?? "Falha ao carregar");
        }
        setLicenses(data.licenses ?? []);
        setAuthed(true);
        window.localStorage.setItem("playmax_admin_key", key);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    },
    [adminKey],
  );

  useEffect(() => {
    const saved = window.localStorage.getItem("playmax_admin_key");
    if (saved) {
      setAdminKey(saved);
      // Com chave salva, carrega a lista direto — sem exigir clique em "Entrar".
      void load(saved);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    <div className="admin-light">
      <main className="admin-shell">
        <h1 className="admin-title">Painel admin — Licenças</h1>
      {authed ? (
        <div className="admin-bar">
          <button
            type="button"
            className="admin-btn primary"
            onClick={() => void createLicense()}
            disabled={loading}
          >
            Nova licença
          </button>
          <button type="button" className="admin-btn" onClick={() => void load()} disabled={loading}>
            Atualizar
          </button>
          <span style={{ flex: 1 }} />
          <button
            type="button"
            className="admin-btn"
            onClick={() => {
              window.localStorage.removeItem("playmax_admin_key");
              setAdminKey("");
              setAuthed(false);
              setLicenses([]);
            }}
          >
            Sair
          </button>
        </div>
      ) : (
        <form
          className="admin-bar"
          onSubmit={(event) => {
            event.preventDefault();
            void load();
          }}
        >
          <input
            type="password"
            className="admin-input"
            placeholder="ADMIN_API_KEY"
            value={adminKey}
            onChange={(event) => setAdminKey(event.target.value)}
          />
          <button type="submit" className="admin-btn primary" disabled={loading}>
            Entrar
          </button>
        </form>
      )}

      {createdKey ? (
        <p className="admin-banner">
          Licença criada: <strong>{createdKey}</strong>
        </p>
      ) : null}
      {error ? <p className="admin-error">{error}</p> : null}

      <table className="admin-table">
        <thead>
          <tr>
            <th>Chave</th>
            <th>Status</th>
            <th>Vencimento</th>
            <th>PCs</th>
            <th>Nota</th>
            <th>Ações</th>
          </tr>
        </thead>
        <tbody>
          {licenses.map((license) => {
            const activations = license.license_activations?.[0]?.count ?? 0;
            return (
              <Fragment key={license.id}>
                <tr>
                  <td className="admin-key">{license.license_key}</td>
                  <td>
                    <span className={`status-pill ${license.status}`}>{license.status}</span>
                  </td>
                  <td>
                    {license.expires_at
                      ? new Date(license.expires_at).toLocaleDateString("pt-BR")
                      : "—"}
                  </td>
                  <td>
                    {activations} / {license.max_devices ?? 1}
                  </td>
                  <td className="admin-note">{license.notes ?? "—"}</td>
                  <td>
                    <div className="admin-actions">
                      {license.status === "revoked" ? (
                        <button
                          type="button"
                          className="admin-btn"
                          disabled={loading}
                          onClick={() => void patchLicense({ licenseId: license.id, action: "unrevoke" })}
                        >
                          Desbloquear
                        </button>
                      ) : (
                        <button
                          type="button"
                          className="admin-btn"
                          disabled={loading}
                          onClick={() => void patchLicense({ licenseId: license.id, action: "revoke" })}
                        >
                          Bloquear
                        </button>
                      )}
                      <button
                        type="button"
                        className="admin-btn"
                        disabled={loading || activations === 0}
                        onClick={() => {
                          if (window.confirm("Resetar a ativação? O PC atual será desconectado.")) {
                            void patchLicense({ licenseId: license.id, action: "reset_devices" });
                          }
                        }}
                      >
                        Resetar PC
                      </button>
                      <button
                        type="button"
                        className="admin-btn"
                        disabled={loading}
                        onClick={() => startEdit(license)}
                      >
                        Editar
                      </button>
                      <button
                        type="button"
                        className="admin-btn danger"
                        disabled={loading}
                        onClick={() => {
                          if (
                            window.confirm(
                              `Apagar a licença ${license.license_key} permanentemente? Esta ação não tem volta.`,
                            )
                          ) {
                            void patchLicense({ licenseId: license.id, action: "delete" });
                          }
                        }}
                      >
                        Apagar
                      </button>
                    </div>
                  </td>
                </tr>
                {editingId === license.id ? (
                  <tr className="admin-edit">
                    <td colSpan={6}>
                      <div className="admin-edit-fields">
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
                            style={{ width: 64 }}
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
                        <button
                          type="button"
                          className="admin-btn primary"
                          disabled={loading}
                          onClick={() => void saveEdit(license.id)}
                        >
                          Salvar
                        </button>
                        <button type="button" className="admin-btn" onClick={() => setEditingId(null)}>
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
    </div>
  );
}

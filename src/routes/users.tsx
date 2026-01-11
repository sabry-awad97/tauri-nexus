import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import {
  useUsers,
  useCreateUser,
  useDeleteUser,
  type User,
} from "../rpc/contract";

function UserCard({
  user,
  onDelete,
  isDeleting,
}: {
  user: User;
  onDelete: () => void;
  isDeleting: boolean;
}) {
  const initials = user.name
    .split(" ")
    .map((n) => n[0])
    .join("")
    .toUpperCase()
    .slice(0, 2);

  const colors = [
    "#6366f1",
    "#8b5cf6",
    "#ec4899",
    "#f43f5e",
    "#f97316",
    "#eab308",
    "#22c55e",
    "#14b8a6",
  ];
  const colorIndex = user.id % colors.length;
  const bgColor = colors[colorIndex];

  return (
    <div className={`user-card ${isDeleting ? "deleting" : ""}`}>
      <div className="user-avatar" style={{ backgroundColor: bgColor }}>
        {initials}
      </div>
      <div className="user-details">
        <span className="user-name">{user.name}</span>
        <span className="user-email">{user.email}</span>
        <span className="user-meta">
          ID: {user.id} • Created:{" "}
          {new Date(user.createdAt).toLocaleDateString()}
        </span>
      </div>
      <button
        onClick={onDelete}
        disabled={isDeleting}
        className="user-delete-btn"
        title="Delete user"
      >
        {isDeleting ? <span className="spinner small" /> : "🗑️"}
      </button>
    </div>
  );
}

function CreateUserForm({ onSuccess }: { onSuccess: () => void }) {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const createUser = useCreateUser({
    onSuccess: () => {
      setName("");
      setEmail("");
      onSuccess();
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !email.trim()) return;
    createUser.mutate({ name, email });
  };

  const isValidEmail = email.includes("@") && email.includes(".");

  return (
    <form onSubmit={handleSubmit} className="create-user-form">
      <h3>Add New User</h3>

      <div className="form-field">
        <label htmlFor="name">Name</label>
        <input
          id="name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="John Doe"
          className="form-input"
        />
      </div>

      <div className="form-field">
        <label htmlFor="email">Email</label>
        <input
          id="email"
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder="john@example.com"
          className={`form-input ${email && !isValidEmail ? "invalid" : ""}`}
        />
        {email && !isValidEmail && (
          <span className="field-error">Please enter a valid email</span>
        )}
      </div>

      {createUser.error && (
        <div className="form-error">
          <span>⚠️</span> {createUser.error.message}
        </div>
      )}

      <button
        type="submit"
        disabled={
          createUser.isPending || !name.trim() || !email.trim() || !isValidEmail
        }
        className="submit-btn"
      >
        {createUser.isPending ? (
          <>
            <span className="spinner small" />
            Creating...
          </>
        ) : (
          <>
            <span>➕</span>
            Add User
          </>
        )}
      </button>
    </form>
  );
}

function UsersPage() {
  const { data: users, isLoading, error, refetch } = useUsers();
  const deleteUser = useDeleteUser({ onSuccess: () => refetch() });
  const [deletingId, setDeletingId] = useState<number | null>(null);

  const handleDelete = async (id: number) => {
    setDeletingId(id);
    try {
      await deleteUser.mutateAsync({ id });
    } finally {
      setDeletingId(null);
    }
  };

  return (
    <div className="page users-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">👥 Users</h1>
          <p className="page-subtitle">
            Full CRUD operations with mutations and optimistic updates
          </p>
        </div>
        <div className="header-stats">
          <span className="stat">
            <strong>{users?.length ?? 0}</strong> users
          </span>
        </div>
      </header>

      <div className="users-layout">
        <aside className="users-sidebar">
          <CreateUserForm onSuccess={refetch} />
        </aside>

        <section className="users-list-section">
          <div className="section-header">
            <h2>All Users</h2>
            <button
              onClick={() => refetch()}
              className="refresh-btn"
              disabled={isLoading}
            >
              {isLoading ? <span className="spinner small" /> : "🔄"} Refresh
            </button>
          </div>

          {isLoading && !users && (
            <div className="loading-state">
              <span className="spinner large" />
              <span>Loading users...</span>
            </div>
          )}

          {error && (
            <div className="error-state">
              <span className="error-icon">⚠️</span>
              <span>{error.message}</span>
              <button onClick={() => refetch()} className="retry-btn">
                Retry
              </button>
            </div>
          )}

          {users && users.length === 0 && (
            <div className="empty-state">
              <span className="empty-icon">👤</span>
              <h3>No users yet</h3>
              <p>Create your first user using the form on the left</p>
            </div>
          )}

          {users && users.length > 0 && (
            <div className="users-grid">
              {users.map((user: User) => (
                <UserCard
                  key={user.id}
                  user={user}
                  onDelete={() => handleDelete(user.id)}
                  isDeleting={deletingId === user.id}
                />
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

export const Route = createFileRoute("/users")({
  component: UsersPage,
});

import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { orpc, useQueryClient, type User } from "../rpc/contract";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Spinner } from "@/components/ui/spinner";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { ScrollArea } from "@/components/ui/scroll-area";

const AVATAR_COLORS = [
  "bg-indigo-500",
  "bg-violet-500",
  "bg-pink-500",
  "bg-rose-500",
  "bg-orange-500",
  "bg-yellow-500",
  "bg-green-500",
  "bg-teal-500",
];

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

  const colorClass = AVATAR_COLORS[user.id % AVATAR_COLORS.length];

  return (
    <div
      className={`flex items-center gap-4 p-4 rounded-lg bg-muted/50 transition-opacity ${isDeleting ? "opacity-50" : ""}`}
    >
      <Avatar className="size-12">
        <AvatarFallback className={`${colorClass} text-white font-semibold`}>
          {initials}
        </AvatarFallback>
      </Avatar>
      <div className="flex-1 min-w-0">
        <p className="font-semibold truncate">{user.name}</p>
        <p className="text-sm text-muted-foreground truncate">{user.email}</p>
        <p className="text-xs text-muted-foreground/70">
          ID: {user.id} • Created:{" "}
          {new Date(user.createdAt).toLocaleDateString()}
        </p>
      </div>
      <Button
        variant="ghost"
        size="icon"
        onClick={onDelete}
        disabled={isDeleting}
        className="hover:bg-destructive/10 hover:text-destructive"
      >
        {isDeleting ? <Spinner className="size-4" /> : "🗑️"}
      </Button>
    </div>
  );
}

function CreateUserForm({ onSuccess }: { onSuccess: () => void }) {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const queryClient = useQueryClient();

  const createUser = useMutation({
    ...orpc.user.create.mutationOptions(),
    onSuccess: () => {
      setName("");
      setEmail("");
      queryClient.invalidateQueries({ queryKey: orpc.user.key() });
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
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Add New User</CardTitle>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="John Doe"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="john@example.com"
              className={email && !isValidEmail ? "border-destructive" : ""}
            />
            {email && !isValidEmail && (
              <p className="text-xs text-destructive">
                Please enter a valid email
              </p>
            )}
          </div>

          {createUser.error && (
            <Alert variant="destructive">
              <AlertDescription className="flex items-center gap-2">
                <span>⚠️</span> {createUser.error.message}
              </AlertDescription>
            </Alert>
          )}

          <Button
            type="submit"
            className="w-full"
            disabled={
              createUser.isPending ||
              !name.trim() ||
              !email.trim() ||
              !isValidEmail
            }
          >
            {createUser.isPending ? (
              <>
                <Spinner className="size-4 mr-2" /> Creating...
              </>
            ) : (
              <>
                <span className="mr-2">➕</span> Add User
              </>
            )}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}

function UsersPage() {
  const {
    data: users,
    isLoading,
    error,
    refetch,
  } = useQuery(orpc.user.list.queryOptions());
  const queryClient = useQueryClient();

  const deleteUser = useMutation({
    ...orpc.user.delete.mutationOptions(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: orpc.user.key() });
      refetch();
    },
  });

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
    <div className="p-8 max-w-6xl mx-auto space-y-8">
      <header className="flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">👥 Users</h1>
          <p className="text-muted-foreground">
            Full CRUD operations with mutations and optimistic updates
          </p>
        </div>
        <p className="text-sm text-muted-foreground">
          <span className="font-semibold text-foreground">
            {users?.length ?? 0}
          </span>{" "}
          users
        </p>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-[320px_1fr] gap-6">
        <aside className="lg:sticky lg:top-8 h-fit">
          <CreateUserForm onSuccess={refetch} />
        </aside>

        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0 pb-4">
            <CardTitle className="text-base">All Users</CardTitle>
            <Button
              variant="outline"
              size="sm"
              onClick={() => refetch()}
              disabled={isLoading}
            >
              {isLoading ? (
                <Spinner className="size-4 mr-2" />
              ) : (
                <span className="mr-2">🔄</span>
              )}
              Refresh
            </Button>
          </CardHeader>
          <CardContent>
            {isLoading && !users && (
              <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                <Spinner className="size-8 mb-4" />
                <span>Loading users...</span>
              </div>
            )}

            {error && (
              <div className="flex flex-col items-center justify-center py-12 gap-3">
                <span className="text-3xl">⚠️</span>
                <span className="text-muted-foreground">{error.message}</span>
                <Button onClick={() => refetch()}>Retry</Button>
              </div>
            )}

            {users && users.length === 0 && (
              <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                <span className="text-5xl mb-4">👤</span>
                <h3 className="font-semibold text-foreground mb-1">
                  No users yet
                </h3>
                <p className="text-sm">Create your first user using the form</p>
              </div>
            )}

            {users && users.length > 0 && (
              <ScrollArea className="h-[500px] pr-4">
                <div className="space-y-3">
                  {users.map((user: User) => (
                    <UserCard
                      key={user.id}
                      user={user}
                      onDelete={() => handleDelete(user.id)}
                      isDeleting={deletingId === user.id}
                    />
                  ))}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export const Route = createFileRoute("/users")({
  component: UsersPage,
});

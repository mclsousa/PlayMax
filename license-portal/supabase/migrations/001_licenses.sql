-- Play Max license portal (run in Supabase SQL editor)

create type public.license_status as enum (
  'pending',
  'active',
  'expired',
  'revoked',
  'trial'
);

create table if not exists public.customers (
  id uuid primary key default gen_random_uuid(),
  name text,
  email text,
  whatsapp text,
  stripe_customer_id text unique,
  created_at timestamptz not null default now()
);

create table if not exists public.licenses (
  id uuid primary key default gen_random_uuid(),
  customer_id uuid references public.customers(id) on delete set null,
  license_key text not null unique,
  status public.license_status not null default 'pending',
  expires_at timestamptz,
  stripe_subscription_id text unique,
  stripe_checkout_session_id text,
  max_devices integer not null default 1 check (max_devices >= 1),
  notes text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.license_activations (
  id uuid primary key default gen_random_uuid(),
  license_id uuid not null references public.licenses(id) on delete cascade,
  device_fingerprint text not null,
  device_name text,
  activated_at timestamptz not null default now(),
  last_validated_at timestamptz not null default now(),
  unique (license_id, device_fingerprint)
);

create index if not exists idx_licenses_key on public.licenses (license_key);
create index if not exists idx_licenses_status on public.licenses (status);
create index if not exists idx_license_activations_license on public.license_activations (license_id);

create or replace function public.touch_license_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

drop trigger if exists licenses_updated_at on public.licenses;
create trigger licenses_updated_at
before update on public.licenses
for each row execute function public.touch_license_updated_at();

alter table public.customers enable row level security;
alter table public.licenses enable row level security;
alter table public.license_activations enable row level security;

-- Supabase (2025+): tables are not auto-exposed to the Data API; grant service_role explicitly.
grant usage on schema public to service_role;
grant select, insert, update, delete on table public.customers to service_role;
grant select, insert, update, delete on table public.licenses to service_role;
grant select, insert, update, delete on table public.license_activations to service_role;

-- API uses service role; no public policies by default.

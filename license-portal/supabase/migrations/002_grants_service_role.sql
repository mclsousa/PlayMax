-- Run this if admin/API returns "permission denied for table licenses"
-- Safe to re-run after 001_licenses.sql

grant usage on schema public to service_role;
grant select, insert, update, delete on table public.customers to service_role;
grant select, insert, update, delete on table public.licenses to service_role;
grant select, insert, update, delete on table public.license_activations to service_role;

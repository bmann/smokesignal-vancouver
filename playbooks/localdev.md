# Localdev Playbook

To run Smoke Signal in localdev (assuming vscode):

1. Create the localdev services https://tangled.sh/@smokesignal.events/localdev

2. Create a Smoke Signal dev container. Ensure it is connected to tailscale.

3. Run migrations `sqlx database reset`

4. Copy `.vscode/launch.example.json` to `.vscode.json` and set the following environment variables:

   * `DNS_NAMESERVERS` to `100.100.100.100`
   * `PLC_HOSTNAME` to `plc.internal.ts.net`. Be sure to change `internal.ts.net` to whatever your Tailnet name is (i.e. `sneaky-fox.ts.net`)
   * `EXTERNAL_BASE` to `placeholder.tunn.dev`. Be sure to change this to whatever tunnel service you're using.

5. Start your developer tunnel `tunnelto --subdomain placeholder --port 3100 --host localhost`

At this point you can open up https://placeholder.tunn.dev/ and login with identities created with https://didadmin.internal.ts.net using the handle and password "password".
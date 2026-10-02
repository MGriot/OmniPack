# OmniPack API

Other programs can use OmniPack's planner over HTTP. An ERP system such as SAP, a
warehouse system, a script or another app sends a container and its cargo and gets back
where every unit goes, in loading order. The answer comes with the same physics checks as
in the app: stability, stacking loads, transport securing (EN 12195-1) and load balance
(CTU Code, VGM, axle loads).

There are three ways in:

| | For | How |
|---|---|---|
| **REST API** | anything that can make an HTTP call | `POST /api/v1/erp/plan` and friends, JSON or CSV back |
| **Jobs + callback** | long searches, asynchronous interfaces | `POST /api/v1/jobs`; poll, or get the result POSTed back |
| **Drop folders** | file-based interfaces (for example an SAP PI/PO file adapter) | put a JSON or CSV file in the inbox, read the result from the outbox |

The full description is the OpenAPI 3.1 document `/api/v1/openapi.json`, which is also
in the repository as `crates/omnipack-api/openapi.json`. Import it into Postman, SAP API
Management or SAP Integration Suite.

## Running it

**In the Windows app (Local API):** click **API…** in the toolbar, then **Enable the
API** → **Apply**.
- It answers on `http://127.0.0.1:8765` while the app is open. Other programs on the same
  computer can use it.
- **Allow other computers** listens on the network and requires an API key. Remember to
  allow the port in the Windows firewall.
- The dialog also sets the drop folders.
- The app's learned placement model is used automatically.
- The phone app does not serve the API.

**As a service (`omnipack-server`):** the same API, without the app. It is a single
executable for Windows (`omnipack-server_<version>_windows-x64.exe`) and Linux
(`omnipack-server_<version>_linux-x64`), also available as a Docker image.

```
omnipack-server                                   # http://127.0.0.1:8765, this computer only
omnipack-server --bind 0.0.0.0:8765 --api-key change-me
omnipack-server --config server.json              # all settings in one file
```

| Flag | Meaning |
|---|---|
| `--bind ADDR:PORT` | where to listen (default `127.0.0.1:8765`; env `OMNIPACK_BIND`) |
| `--api-key KEY` | accepted key, repeatable (env `OMNIPACK_API_KEYS=key1,key2`) |
| `--insecure-no-auth` | listen on the network without a key (trusted networks only) |
| `--tls-cert cert.pem --tls-key key.pem` | HTTPS (otherwise put a reverse proxy in front) |
| `--cors ORIGIN` | allow browser apps from this origin (`*` = any) |
| `--inbox DIR --outbox DIR [--archive DIR]` | drop folders |
| `--drop-container PRESET\|file.json`, `--drop-options file.json`, `--drop-units mm,kg` | container, options and units for CSV drop files |
| `--ranker model.json` | learned placement model (`omnipack train`, or the app's `model.json`) |
| `--max-sync-seconds S` / `--max-job-seconds S` | longest search for a direct call (60) / for a job (600) |
| `--max-jobs N` | jobs running at the same time (2; the others wait) |
| `--quiet` | no request log |

`server.json` holds the same settings, for example:

```json
{ "bind": "0.0.0.0:8765", "api_keys": ["change-me"], "max_jobs": 4,
  "drop": { "inbox": "D:/omnipack/in", "outbox": "D:/omnipack/out", "container": { "preset": "40ft-hc" } } }
```

**Docker:**

```
docker build -t omnipack-server .
docker run -d -p 8765:8765 -e OMNIPACK_API_KEYS=change-me omnipack-server
```

**As a Windows service:** with [NSSM](https://nssm.cc), run
`nssm install OmniPack C:\omnipack\omnipack-server.exe --bind 0.0.0.0:8765 --api-key change-me`.

**As a Linux service:** a systemd unit with
`ExecStart=/opt/omnipack/omnipack-server --config /etc/omnipack/server.json` and
`Restart=on-failure`.

## Authentication and limits

- **API keys.** Send the key as `X-API-Key: <key>` or `Authorization: Bearer <key>`.
  - `/api/v1/health` and `/api/v1/openapi.json` are open.
  - Without any key configured, the server only listens on this computer.
- **HTTPS.** Use `--tls-cert/--tls-key`, or a reverse proxy (IIS, nginx, Caddy), or SAP
  Cloud Connector in front.
- **Limits.**
  - Bodies up to 10 MB and 20 000 units per request.
  - A direct search runs at most 60 s; use a job for longer ones.
  - Finished jobs are kept for 24 hours.
- **Errors.** They come back as `{ "error": "…" }`: 400 (bad request), 401 (key),
  404, 413 (too large).

## Endpoints

All paths start with `/api/v1`.

| Method | Path | What |
|---|---|---|
| GET | `/health` | `{status, version, jobs_active, learned_model}` |
| GET | `/presets` | container presets, road vehicles, transport legs |
| GET | `/openapi.json` | the API description |
| POST | `/erp/plan` | **plan a load, simple format** (below) |
| POST | `/pack` | plan a load, full OmniPack JSON (`PackRequest` → `PackResult`, mm and kg; see [conventions.md](conventions.md)) |
| POST | `/optimize` | ★ Best search, full format: `{ request, budget_s, options }` → up to 3 plans |
| POST | `/validate` | check a plan made elsewhere |
| POST | `/jobs` | start a background job → 202 + `Location` |
| GET | `/jobs`, `/jobs/{id}` | list jobs; one job with progress and, when done, its result |
| DELETE | `/jobs/{id}` | cancel (a search keeps the best plan so far) |

`/erp/plan` and `/pack` return the CSV load list instead of JSON with `?format=csv` or
`Accept: text/csv`.

## The simple format (`/erp/plan`)

```json
{
  "reference": "80001234",
  "units": { "length": "CM", "weight": "KG" },
  "container": { "preset": "40ft-hc" },
  "items": [
    { "id": "MAT-1001", "description": "Pallet, machine parts", "quantity": 6,
      "length": 120, "width": 80, "height": 100, "weight": 450, "this_side_up": true, "max_load_on_top": 1000 },
    { "id": "MAT-2002", "quantity": 20, "length": 60, "width": 40, "height": 50, "weight": 25, "fragile": true },
    { "id": "MAT-3003", "quantity": 4, "shape": "cylinder", "diameter": 58, "height": 88, "weight": 180 }
  ],
  "options": { "fill": "walls", "transport": ["road", "sea_b"] }
}
```

**Units.**
- Lengths: `mm`, `cm`, `m`, `in`, `ft`, or the SAP/ISO codes `MMT`, `CMT`, `MTR`, `INH`,
  `FOT`.
- Weights: `kg`, `g`, `t`, `lb`, or `KGM`, `GRM`, `TNE`/`TO`, `LBR`.
- The answer uses the same units. The default is mm and kg.

**Container.**
- `preset`: one of `20ft-dv`, `40ft-dv`, `40ft-hc` or `semi-trailer-13.6`.
- Or the inside sizes: `length`, `width`, `height`. Inside sizes override the preset.
- Optional: `max_payload`, `door_width`, `door_height`, `tare`, `floor_rating` (kg/m²).
- `vehicle` takes a road vehicle preset name, for axle loads.

**Items.**
- Required: `id` and `weight` (gross, per unit). `quantity` defaults to 1.
- `shape`: `box` (default) needs `length`, `width` and `height`. A `cylinder` is a
  standing drum: `diameter` and `height`. A `sphere` needs `diameter`.
- Flags: `stackable` (default true; false = nothing on top), `max_load_on_top` (weight),
  `fragile`, `this_side_up`, `floor_only`.
- `stop`: 1 is unloaded first. `zone`: `any`, `front` (near the door) or `back`.
- `friction`.

**Options** (all optional):

| Option | Values |
|---|---|
| `fill` | `walls` (default), `floor`, `length`, `rows`, `corner`, `learned`, or `best`, which searches for `search_seconds` (default 10) |
| `transport` | any of `road`, `rail`, `rail_shunting`, `sea_a`, `sea_b`, `sea_c` (default `["road"]`) |
| `stop_order` | `lifo` (default) or `fifo` |
| `rotation` | `true` (default) |
| `centre_lengthwise`, `ctu_checks` | booleans |
| `dunnage_mm` | largest gap filled with dunnage (default 50) |
| `friction` | default friction (0.4) |
| `max_containers` | number |
| `advanced` | any [PackOptions](conventions.md) field, merged last |

**Answer:**

```json
{
  "reference": "80001234", "status": "ok", "units": { "length": "CM", "weight": "KG" },
  "summary": { "units_requested": 30, "units_placed": 30, "containers": 1, "volume_utilization": 0.119,
               "total_weight": 3920, "valid": true, "balance_warnings": 3, "units_to_lash": 30 },
  "containers": [ { "index": 1, "id": "40ft-hc-1", "length": 1203.2, "width": 235.2, "height": 269.8,
                    "units": 30, "weight": 3920, "volume_utilization": 0.119, "center_of_gravity": [102.1, 45.2, 103.9],
                    "vgm": 7820, "violations": [], "warnings": ["centre of gravity 497.7 CM towards the front wall (limit 60.2 CM)", "…"],
                    "transport": [ { "case": "Road (EN 12195-1)", "issues": ["MAT-1001#1: sliding left at 0.5 g, block ≥ 0.95 kN or 1 lashing(s)"], "dunnage": 12.5 } ] } ],
  "placements": [
    { "container": 1, "sequence": 1, "item_id": "MAT-1001", "unit": 1, "x": 0, "y": 0, "z": 0,
      "length": 80, "width": 120, "height": 100, "rotation": "DHW", "on_floor": true,
      "load_on_top": 513.7, "securing": "lashing", "lashings": 1 }
  ],
  "not_placed": []
}
```

**Status values:**
- `ok`: everything placed.
- `partial`: some units are listed in `not_placed`, with a reason such as `too_large`,
  `door_too_small`, `no_stable_position` or `container_limit`.
- `invalid`: only from `/validate`; the plan has violations.

**Placements:**
- `sequence` is the loading order in each container.
- `x`, `y`, `z` are the minimum corner of the unit's box.
  - `x` runs across the width from the left wall, `y` up from the floor, and `z` along
    the length from the front wall towards the door.
- `length`, `width` and `height` are the unit's size as placed, along those axes.
- `rotation` says which of the item's own dimensions lies along the container's width,
  height and length:
  - `WHD`: as defined;
  - `DHW`: turned 90° on the floor;
  - `HWD`, `WDH`, `HDW`, `DWH`: tipped over.
- `securing`: `secured`, `dunnage` (held once the listed gaps are filled), `chocks`,
  `lashing` (needs lashing or blocking; `lashings` gives the number of direct lashings)
  or `overloaded`.

**Warnings and violations:**
- `warnings` are load-balance notes (CTU Code window, middle-half share, CoG height,
  vehicle axles, floor pressure). They do not make the plan invalid.
- `violations` (mm and kg) are physical problems. Plans OmniPack makes have none.

## Checking a plan made elsewhere (`/validate`)

Send a simple-format request with `placements`, in its units:
`[{ "item_id": "PAL", "x": 0, "y": 0, "z": 0, "rotation": "WHD" }, …]`. Every unit is put
exactly there, with no gravity and no moving, and the plan is checked.
- `status` is `invalid` and `violations` lists what is wrong: overlap, unsupported,
  unstable, overloaded, door too small, …
- It also gives the securing, balance and load list, as for a plan.

The full format takes `{ request: PackRequest, placements: [{ item_id, orientation,
position: [x, y, z] }] }` (mm) and returns a `ContainerPlan`.

## Jobs and callbacks

```json
POST /api/v1/jobs
{ "kind": "best", "budget_s": 30, "reference": "80001234",
  "erp": { …simple-format request… },
  "callback": { "url": "https://my-tenant.it-cpi.cfapps.eu10.hana.ondemand.com/http/omnipack/result",
                "headers": { "Authorization": "Basic …" } } }
```

- **Request.** `kind` is `pack` (one pass) or `best` (search for `budget_s` seconds, at
  most 600). Use `erp` for the simple format, or `request` (with optional `options`) for
  the full format.
- **Answer.** `202` with `{ id, status: "queued", … }` and `Location: /api/v1/jobs/{id}`.
- **Polling.** `GET /api/v1/jobs/{id}` shows `status`: `queued`, `running`, `done`,
  `failed` or `cancelled`. While a search runs it shows `progress`; when done, the
  `result` (simple-format answer, `PackResult` or `OptimizeResult`).
- **Callback.** The finished job is POSTed to `callback.url` as `{ job_id, reference,
  status, error, result }`, with your `headers`. If it fails, it is retried 3 times
  (after 1, 4 and 9 s). The delivery state is in the job's `callback` field.
- **Cancel.** `DELETE /api/v1/jobs/{id}` cancels; a search stops and keeps the best plan
  it found.

## Drop folders

Put files in the **inbox**:
- `*.json`: a full or simple-format request (the container and options inside).
- `*.csv`: an item list with a header row. The container, units and options come from
  the drop settings (default: a 40 ft high cube, mm and kg).
  - Columns: `id` (or `material`, `matnr`), `description`, `quantity`, `length`,
    `width`, `height`, `weight`, and optionally `shape`, `diameter`, `stackable`,
    `max_load_on_top`, `fragile`, `this_side_up`, `floor_only`, `stop`, `zone`,
    `friction`.
  - The SAP field names work as column names too: `MATNR`, `MAKTX`, `LFIMG` or
    `MENGE`, `LAENG`, `BREIT`, `HOEHE`, `BRGEW`. An export of the delivery items
    can go into the inbox as it is.
  - The separator is `,` or `;`. Decimal commas are fine. Booleans can be `1/0`,
    `true/false`, `yes/no` or `X`.

Every 2 seconds OmniPack processes files that have not changed for a second.
- **Success:** the **outbox** receives `<name>.result.json` (the answer) and
  `<name>.loadlist.csv`.
- **Failure:** the outbox receives `<name>.error.txt`.
- Files are written under a temporary name and then renamed, so an adapter never picks
  up half a file.
- The input moves to `archive/` (by default inside the inbox), so it is never processed
  twice.

## SAP integration

**Field mapping** (delivery and material master):

| OmniPack | SAP |
|---|---|
| `reference` | `LIKP-VBELN` (delivery) or `VTTK-TKNUM` (shipment) |
| `items[].id` | `LIPS-MATNR` |
| `items[].description` | `MAKT-MAKTX` |
| `items[].quantity` | `LIPS-LFIMG` (in base units, one unit = one package) |
| `items[].length / width / height` | `MARM-LAENG / BREIT / HOEHE` (packaging unit) |
| `units.length` | `MARM-MEABM` (ISO code, e.g. `MMT`, `CMT`) |
| `items[].weight` | `MARM-BRGEW` (gross weight) |
| `units.weight` | `MARM-GEWEI` (e.g. `KGM`) |
| `items[].stop` | sequence of the delivery in the shipment |

**From ABAP** (`cl_http_client` and `/ui2/cl_json`, simplified):

```abap
DATA(lv_json) = /ui2/cl_json=>serialize( data = ls_request pretty_name = /ui2/cl_json=>pretty_mode-low_case ).
cl_http_client=>create_by_url( EXPORTING url = 'https://omnipack.example.local/api/v1/erp/plan'
                               IMPORTING client = DATA(lo_http) ).
lo_http->request->set_method( 'POST' ).
lo_http->request->set_header_field( name = 'Content-Type' value = 'application/json' ).
lo_http->request->set_header_field( name = 'X-API-Key'    value = lv_key ).
lo_http->request->set_cdata( lv_json ).
lo_http->send( ). lo_http->receive( ).
/ui2/cl_json=>deserialize( EXPORTING json = lo_http->response->get_cdata( ) CHANGING data = ls_response ).
" ls_response-placements: position and loading sequence per unit; write them back to the
" handling units, the shipment, or a custom table.
```

Keep the key in a secure store (SSF or a credentials table), not in the code. In S/4HANA
Cloud, use the communication arrangement / `cl_web_http_client_manager` instead of
`cl_http_client`.

**SAP Integration Suite (Cloud Integration) / API Management:**
- Import `openapi.json` as an API provider.
- In an iFlow, map the delivery IDoc or OData payload to the simple format, then call
  `/erp/plan` with an HTTP receiver adapter. Put the API key in a header from the
  credential store.
- For long searches, create a job with the iFlow's own HTTPS endpoint as the
  `callback.url` and its credentials in `callback.headers`.
- Reach an on-premise OmniPack server through Cloud Connector.

**SAP PI/PO or file interfaces:** let the file adapter write the item CSV (or JSON) into
the inbox and pick up `*.result.json` / `*.loadlist.csv` from the outbox.

## Quick test

```
curl http://127.0.0.1:8765/api/v1/health
curl -X POST http://127.0.0.1:8765/api/v1/erp/plan -H "X-API-Key: change-me" -H "Content-Type: application/json" -d @delivery.json
curl -X POST "http://127.0.0.1:8765/api/v1/erp/plan?format=csv" -H "X-API-Key: change-me" -H "Content-Type: application/json" -d @delivery.json -o loadlist.csv
```

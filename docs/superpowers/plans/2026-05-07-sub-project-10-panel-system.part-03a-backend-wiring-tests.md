### Step 3.3: Wire into `main.rs`

- [ ] **Step 3.3.1: Modify `services/api/src/main.rs` mod declaration**

Edit lines 53-60 (the `mod routes { ... }` block) to add:

```rust
mod routes {
    pub mod admin_listings;
    pub mod auth_event;
    pub mod bookmarks;
    pub mod buildings;       // SP10 T3
    pub mod health;
    pub mod listings;
    pub mod notifications;
    pub mod parcels;         // SP10 T3
}
```

- [ ] **Step 3.3.2: Add state assembly + router merge**

After line 297 (`listings_router` block end), before line 299 (`// SP6-v: 공유 repository`), add:

```rust
    // SP10 T3: Panel system backing endpoints — pure REST resource.
    let parcels_state = routes::parcels::ParcelsState {
        parcel_lookup: listings_state.parcel_lookup.clone(),
    };
    let parcels_router: Router<()> = Router::new()
        .route("/api/parcels/:pnu", get(routes::parcels::get_parcel))
        .with_state(parcels_state)
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    // SP10 T3: building_register reader 주입 — SP4-iii-a 의 reader 인스턴스화.
    // 미구현 시 (DATA_GO_KR_API_KEY 미설정) NoOp fallback — 빈 list 반환.
    let building_reader: Arc<dyn routes::buildings::BuildingRegisterReader> =
        Arc::new(NoOpBuildingRegisterReader);
    let buildings_state = routes::buildings::BuildingsState { reader: building_reader };
    let buildings_router: Router<()> = Router::new()
        .route("/api/buildings", get(routes::buildings::list_buildings))
        .with_state(buildings_state)
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));
```

Then in the final `app` builder (line 383-389), add `.merge(parcels_router).merge(buildings_router)`:

```rust
    let app = public
        .merge(protected)
        .merge(listings_router)
        .merge(parcels_router)         // SP10 T3
        .merge(buildings_router)       // SP10 T3
        .merge(bookmarks_router)
        .merge(admin_router)
        .merge(notifications_router)
        .merge(internal)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(http::request_id::request_id_layer));
```

- [ ] **Step 3.3.3: Add NoOp building reader stub**

At top of `main.rs` (after `use ...` block), add:

```rust
/// SP10 T3: NoOp building reader — DATA_GO_KR_API_KEY 미설정 시 fallback (빈 list).
/// production 은 SP4-iii-a 의 live reader 로 swap.
struct NoOpBuildingRegisterReader;

impl routes::buildings::BuildingRegisterReader for NoOpBuildingRegisterReader {
    fn list_by_pnu<'a>(
        &'a self,
        _pnu: &'a shared_kernel::pnu::Pnu,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<routes::buildings::BuildingItem>>> + Send + 'a>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}
```

- [ ] **Step 3.3.4: Run cargo check**

Run: `cargo check -p api`
Expected: clean.

- [ ] **Step 3.3.5: Run cargo clippy**

Run: `cargo clippy -p api --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 3.3.6: Commit**

```bash
git add services/api/src/main.rs
git commit -m "feat(sp10-t3): wire /api/parcels/:pnu + /api/buildings into main router"
```

### Step 3.4: Integration test

- [ ] **Step 3.4.1: Create `services/api/tests/sp10_panel_endpoints.rs`**

```rust
//! SP10 T3: panel backing endpoints integration test.

#[tokio::test]
async fn get_parcel_returns_404_for_unknown_pnu() {
    // ... reuses test scaffolding from existing tests/listing_*.rs
    // — minimum: assert that GET /api/parcels/{19-zeros} with NoOp lookup returns 404
    //   (NoOpParcelInfoLookup returns Ok(None) for any pnu).
    //
    // 실제 test scaffold 는 기존 services/api/tests/*.rs (예: listing_search.rs) 의 setup
    // helper 와 동일 패턴 — Axum app 부팅 + tokio::spawn + reqwest call.
    //
    // 빈 stub 으로 시작 — 첫 fail 후 scaffold 복붙해서 채워나감 (TDD red).
    panic!("write me");
}

#[tokio::test]
async fn get_parcel_returns_400_for_invalid_pnu() {
    panic!("write me");
}

#[tokio::test]
async fn list_buildings_returns_empty_with_noop_reader() {
    panic!("write me");
}
```

- [ ] **Step 3.4.2: Fill scaffold by copying from `services/api/tests/listing_search.rs`**

Read the existing test file pattern and replicate the bootstrap (axum app, port 0, reqwest call). Implement the 3 tests above with concrete asserts. Use `Pnu` constructor for valid 19-digit PNU.

- [ ] **Step 3.4.3: Run integration test**

Run: `cargo test -p api --test sp10_panel_endpoints`
Expected: 3 tests pass.

- [ ] **Step 3.4.4: Commit**

```bash
git add services/api/tests/sp10_panel_endpoints.rs
git commit -m "test(sp10-t3): integration tests for /api/parcels/:pnu + /api/buildings (NoOp path)"
```

---


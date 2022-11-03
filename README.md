# rusty-todos

An todo app to sharpen my rusty sword.

## Cargo Watch

```sh
cargo watch -q -c -w src/ -x 'test model_db_ -- --test-threads=1 --nocapture'
```

## DB

```sh
# start the database
docker run --rm -p 5432:5432 -e "POSTGRES_PASSWORD=postgres" --name pg postgres:14

# optional psql (other terminal)
docker exec -it -u postgres pg psql 
```

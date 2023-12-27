
init-kafka-topics:
	docker exec -it pragma-kafka "kafka-topics" "--bootstrap-server" "localhost:9092" "--topic" "pragma-data" "--create" "--partitions" "1" "--replication-factor" "1" || true
	docker exec -it pragma-kafka "kafka-topics" "--bootstrap-server" "localhost:9092" "--topic" "__consumer_offsets" "--create" "--partitions" "1" "--replication-factor" "1" || true
format:
	cargo fmt -- --check
	cargo clippy --no-deps -- -D warnings
	cargo clippy --tests --no-deps -- -D warnings
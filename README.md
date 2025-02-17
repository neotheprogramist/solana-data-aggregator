# Solana Data Aggregator

Solana Data Aggregator is a tool designed to fetch and store Solana blockchain transactions in a **SurrealDB** database and provide a REST API to query the stored transactions. The aggregator ensures efficient and accurate data retrieval while maintaining resilience and scalability.

## 🚀 Getting Started

### 1️⃣ Start SurrealDB

You can start SurrealDB using **Docker Compose** or **Podman**:

#### Using Podman:
```sh
podman run --rm --pull always --name surrealdb -p 8000:8000 surrealdb/surrealdb:latest start --log trace --user root --pass root memory
```

#### Using Docker:
```sh
docker run --rm --pull always --name surrealdb -p 8000:8000 surrealdb/surrealdb:latest start --log trace --user root --pass root memory
```

### 2️⃣ Run the Application

Once the database is running, start the Solana Data Aggregator with:
```sh
cargo run
```

### 3️⃣ Fetch Data

You can query transactions via **cURL**:

#### Fetch transactions by **day**:
```sh
curl "http://localhost:8080/transactions?day=2025-02-17"
```

#### Fetch transaction by **ID**:
```sh
curl "http://localhost:8080/transactions?id=61jvJQWrUBwPtDtkejP4MhR4aiEW9BiKRJC9Cu6s7N4sbU4ngjHrstRmvf1RaadP4p9W8oU5HFmuvcZB5f8jrA3j"
```

### 4️⃣ Explore the Database

To interact with and explore the database, install or use the **web version** of **Surrealist**:
👉 [Surrealist Web App](https://surrealdb.com/surrealist)

---

## 📌 Evaluation Criteria

### ✅ **Functionality**
The Solana Data Aggregator accurately retrieves and processes **on-chain transaction data** from the Solana blockchain. It efficiently stores data in **SurrealDB** and exposes a simple REST API for querying transactions based on **date** or **transaction ID**.

### ✅ **Performance**
**How well does the application handle large volumes of data and concurrent requests?**
The biggest challenges are **API rate limits** and **network speed**. To mitigate this, the fetched transactions are limited by `TX_LIMIT` to avoid processing an entire block at once. This makes the tool suitable for demonstration purposes while ensuring stable performance.

### ✅ **Reliability**
**Is the data aggregator resilient to failures and capable of recovering gracefully?**
Yes. The aggregator automatically retrieves the **last processed slot** from the database and resumes fetching from that point, ensuring **data consistency** and preventing data loss in case of failures.

### ✅ **Scalability**
**Can the application scale to handle increasing data loads without sacrificing performance?**
Yes. The primary constraints are **network speed** and **RPC rate limits**, but the architecture supports **horizontal scaling** by running multiple instances to distribute load efficiently.

### ✅ **Security**
**Are proper security measures implemented to protect data integrity?**
- **SurrealDB authentication** is enabled with **root user & password protection**.
- **Parameterized queries** are used to prevent **SQL injection attacks**.
- Transaction data is securely **fetched, stored, and accessed** without modification risks.

### ✅ **Documentation & Maintainability**
- **Well-structured codebase** with clear module separation.
- **Comprehensive logging** for debugging and monitoring.
- **Easy setup and deployment**, making it **developer-friendly**.
- This README provides **clear installation and usage instructions**.

---

## 📖 License
This project is open-source and licensed under the **MIT License**.

For contributions, issues, or feature requests, visit the GitHub repository: **[Solana Data Aggregator](https://github.com/neotheprogramist/solana-data-aggregator)**.

Happy coding! 🚀

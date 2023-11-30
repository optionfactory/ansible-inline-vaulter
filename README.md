# README

Piccolo tool per poter vedere i secrets vaultati _inline_ all'interno di un file, senza doverli svaultare uno a uno o dover usare l'estensione di vscode (che modifica il file con il secret in chiaro e poi si rischia di committarlo).

## Todo

- ~~Vault come argomento opzionale, se non c'è si sgrufola nell'ansible.cfg~~
  - ~~Solo vault_password_file~~
- Andare a prendersi il file giusto in base al vault id nel campo da decrittare
- In alternativa al file con i secrets, poter specificare l'inventory
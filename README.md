# README

Piccolo tool per poter vedere i secrets vaultati _inline_ all'interno di un file, senza doverli svaultare uno a uno o dover usare l'estensione di vscode (che modifica il file con il secret in chiaro e poi si rischia di committarlo).

Lo script permette di scaricare l'ultima versione del binario da github.

Il token è fine-grained per permettere solo l'accesso readonly alle actions di questa repo.

## Todo

- Potergli passare il path di progetto così uno se lo mette in bin e non ci pensa più
- ~~Vault come argomento opzionale, se non c'è si sgrufola nell'ansible.cfg~~
  - ~~Solo vault_password_file~~
- ~~Andare a prendersi il file giusto in base al vault id nel campo da decrittare~~
- In alternativa al file con i secrets, poter specificare l'inventory
  - Atm considera solo group_vars
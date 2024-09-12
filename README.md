# README

Piccolo tool per poter vedere i secrets vaultati _inline_ all'interno di un file, senza doverli svaultare uno a uno o dover usare l'estensione di vscode (che modifica il file con il secret in chiaro e poi si rischia di committarlo).

Lo script permette di scaricare l'ultima versione del binario da github.

Il token è fine-grained per permettere solo l'accesso readonly alle actions di questa repo.

## Todo

- Supporta host vars
- Migliorare messaggio di errore nel caso che i file specificati in ansible.cfg non esistano

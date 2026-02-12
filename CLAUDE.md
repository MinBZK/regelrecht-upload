# Uploadportal
We bouwen een portal waar teams die hun interne werkprocessen willen vertalen naar regels hun interne regelsets kunnen delen en uploaden.

# Requirements aanvragersportaal
## Functioneel
- [x] Gebruikers moeten hun naam en organisatie en organisatiedeel toevoegen aan hun submission
- [x] Moet verschillende upload categorieen van documenten faciliteren:
    1. formele wetten (een link naar de versie op wetten.overheid.nl)
    2. Circulaire
    3. Uitvoeringsbeleid
    4. werkinstructies
- [x] Het portaal moet mogelijk maken om verschillende classificaties van documenten te uploaden:
    1. Mag publiek op internet gepubliceerd worden
    2. Mag gebruikt worden om te uploaden middels claude code
    3. Je mag het niet eens in claude stoppen
    Deze categorie moet teruggeven aan de gebruiker dat deze dan niet gebruikt kan worden en moet dan ook afgewezen worden
- [x] Zorg dat er een organisatie is van submissions op basis van een slug ID
- [x] Er moet een FAQ zijn waarin details staan over wat we gaan doen met hun beleid en regels

- [ ] Toekomst / low priority: wanneer alle uploads compleet zijn kan de aanvrager een meeting inplannen

## UI vormgeving
- [ ] Gebruikt de RegelRecht Storybook (zie de MVP repo) voor de vormgeving van de UI


## Technisch
- [x] Het moet draaien als podman container
- [x] Voeg Github Actions toe die de container ook publisht als URL. Dus publish als image
- [x] Gebruik RUST (API endpoint), html en Postgres (spin dev database op voor nu, in productie is er een managed Postgres Database).

### Build Requirements
- **Rust version**: 1.85 (image: `rust:1.85-bookworm`)
- **Container runtime**: Podman or Docker
- **Database**: PostgreSQL 16+

### Dependency Pinning
Het Containerfile bevat `cargo update --precise` commando's om transitive dependencies te pinnen naar Rust 1.85-compatibele versies:
- `home@0.5.9` (nieuwere versies vereisen Rust 1.88+)
- `getrandom@0.2.15`

### Running locally
```bash
# Start development environment (database + app)
podman-compose up -d

# Of bouw container direct
podman build -f Containerfile -t regelrecht-upload .

# Verificatie
curl http://localhost:8080/api/faq
```

### Bekende fixes
1. **Migration SQL parsing**: PL/pgSQL functies met `$$ ... $$` blocks worden correct geparsed
2. **Healthcheck**: `curl` is geïnstalleerd in runtime image voor healthcheck


## Juridisch
- [x] stel een privacyverklaring op volgens geldend nederlands recht over dataverwerking. neem daarin op dat de verwerkingstermijn zolang duurt als nodig voor de PoC met hun beleid. zorg dat je hier een goede juridische test op doet en stel me vragen over zaken die je moet invullen.
- [ ] Stel een DPIA op op basis van het ontwerp van het proces/project volgens de rijksstandaard DPIA


# Requirements beheerdersportaal
## Functioneel
- [x] Zorg dat er een overzicht van submissions met details is uit de database voor beheerders (achter beveiligd inlogportaal)
- [x] Er moet zoals bovenstaand dus API endpoint zijn voor het doorzetten van submissions
- [x] Er moet een beheerders agenda zijn waar beschikbare tijdsloten voor het inplannen van meetings aangegeven moeten worden

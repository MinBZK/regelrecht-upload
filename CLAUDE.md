# Uploadportal
We bouwen een portal waar teams die hun interne werkprocessen willen vertalen naar regels hun interne regelsets kunnen delen en uploaden.

# Requirements aanvragersportaal
## Functioneel
- [ ] Gebruikers moeten hun naam en organisatie en organisatiedeel toevoegen aan hun submission
- [ ] Moet verschillende upload categorieen van documenten faciliteren:
    1. formele wetten (een link naar de versie op wetten.overheid.nl)
    2. Circulaire
    3. Uitvoeringsbeleid
    4. werkinstructies
- [ ] Het portaal moet mogelijk maken om verschillende classificaties van documenten te uploaden: 
    1. Mag publiek op internet gepubliceerd worden
    2. Mag gebruikt worden om te uploaden middels claude code
    3. Je mag het niet eens in claude stoppen
    Deze categorie moet teruggeven aan de gebruiker dat deze dan niet gebruikt kan worden en moet dan ook afgewezen worden
- [ ] Zorg dat er een organisatie is van submissions op basis van een slug ID
- [ ] Er moet een FAQ zijn waarin details staan over wat we gaan doen met hun beleid en regels 

- [ ] Toekomst / low priority: wanneer alle uploads compleet zijn kan de aanvrager een 

## UI vormgeving
- [ ] Gebruikt de RegelRecht Storybook (zie de MVP repo) voor de vormgeving van de UI


## Technisch 
- [ ] Het moet draaien als podman container
- [ ] Voeg Github Actions toe die de container ook publisht als URL. Dus publish als image
- [ ] Gebruik RUST (API endpoint), html en Postgres (spin dev database op voor nu, in productie is er een managed Postgres Database).


## Juridisch
- [ ] stel een privacyverklaring op volgens geldend nederlands recht over dataverwerking. neem daarin op dat de verwerkingstermijn zolang duurt als nodig voor de PoC met hun beleid. zorg dat je hier een goede juridische test op doet en stel me vragen over zaken die je moet invullen.
- [ ] Stel een DPIA op op basis van het ontwerp van het proces/project volgens de rijksstandaard DPIA


# Requirements beheerdersportaal
## Functioneel
- [ ] Zorg dat er een overzicht van submissions met details is uit de database voor beheerders (achter beveiligd inlogportaal)
- [ ] Er moet zoals bovenstaand dus API endpoint zijn voor het doorzetten van submissions
- [ ] Er moet een beheerders agenda zijn waar beschikbare tijdsloten voor het inplannen van meetings aangegeven moeten worden

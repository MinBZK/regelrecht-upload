# Data Protection Impact Assessment (DPIA)
## RegelRecht Upload Portal

**Versie**: 1.0
**Datum**: Februari 2024
**Opgesteld door**: RegelRecht Team
**Verwerkingsverantwoordelijke**: Ministerie van Binnenlandse Zaken en Koninkrijksrelaties
**Status**: Concept - Ter beoordeling FG

---

## 1. Inleiding en doel

### 1.1 Aanleiding
Dit document beschrijft de Data Protection Impact Assessment (DPIA) voor de RegelRecht Upload Portal. De DPIA is opgesteld conform het Rijksmodel DPIA en de vereisten van de Algemene Verordening Gegevensbescherming (AVG).

### 1.2 Doel van de DPIA
Het in kaart brengen van privacy-risico's verbonden aan de verwerking van persoonsgegevens via de RegelRecht Upload Portal, en het identificeren van maatregelen om deze risico's te beperken.

### 1.3 Scope
- RegelRecht Upload Portal (web applicatie)
- Proof of Concept periode (maximaal 12 maanden)
- Verwerking van persoonsgegevens van indieners en hun organisaties

---

## 2. Beschrijving van de verwerking

### 2.1 Procesbeschrijving

De RegelRecht Upload Portal faciliteert het volgende proces:

```
┌──────────────────────────────────────────────────────────────────┐
│  1. INDIENING                                                     │
│  ┌─────────────┐    ┌──────────────┐    ┌────────────────┐       │
│  │ Gebruiker   │───>│ Portaal      │───>│ Database       │       │
│  │ vult form   │    │ valideert    │    │ opslag         │       │
│  └─────────────┘    └──────────────┘    └────────────────┘       │
├──────────────────────────────────────────────────────────────────┤
│  2. VERWERKING                                                    │
│  ┌─────────────┐    ┌──────────────┐    ┌────────────────┐       │
│  │ Admin       │───>│ Beoordeling  │───>│ Doorsturen     │       │
│  │ review      │    │ documenten   │    │ naar team      │       │
│  └─────────────┘    └──────────────┘    └────────────────┘       │
├──────────────────────────────────────────────────────────────────┤
│  3. GESPREK                                                       │
│  ┌─────────────┐    ┌──────────────┐    ┌────────────────┐       │
│  │ Slot        │───>│ Meeting      │───>│ Resultaat      │       │
│  │ boeken      │    │ met experts  │    │ terugkoppeling │       │
│  └─────────────┘    └──────────────┘    └────────────────┘       │
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 Betrokken persoonsgegevens

| Categorie | Gegevens | Herkomst | Bewaartermijn |
|-----------|----------|----------|---------------|
| Identificatie | Naam | Gebruiker | 12 maanden |
| Contact | E-mail (optioneel) | Gebruiker | 12 maanden |
| Organisatie | Organisatienaam, afdeling | Gebruiker | 12 maanden |
| Documenten | Geuploade bestanden | Gebruiker | 12 maanden |
| Technisch | IP-adres, sessietokens | Systeem | 12 maanden |
| Audit | Verwerkingslog | Systeem | 12 maanden |

### 2.3 Categorieën betrokkenen
- **Indieners**: Medewerkers van overheidsorganisaties die documenten indienen
- **Experts**: Medewerkers die deelnemen aan gesprekken
- **Beheerders**: Medewerkers van MinBZK die de portal beheren

### 2.4 Ontvangers
- RegelRecht projectteam (MinBZK)
- IT-beheerders (verwerkers, onder verwerkersovereenkomst)
- Toezichthouders (indien wettelijk vereist)

---

## 3. Noodzaak en proportionaliteit

### 3.1 Doelbinding
De verwerking is noodzakelijk voor:
1. **Primair doel**: Uitvoeren van de PoC voor machine-leesbare regels
2. **Secundair doel**: Communicatie met indieners over hun inzending
3. **Hulpdoel**: Beveiliging en audit van het systeem

### 3.2 Rechtmatigheid

| Rechtsgrond | Toepassing | Onderbouwing |
|-------------|------------|--------------|
| Toestemming (art. 6(1)(a)) | Ja | Expliciete toestemming bij indiening |
| Gerechtvaardigd belang (art. 6(1)(f)) | Ja | Onderzoek naar betere regelgeving |

### 3.3 Proportionaliteit
- **Minimale gegevensverzameling**: Alleen noodzakelijke gegevens worden gevraagd
- **Beperkte bewaartermijn**: Maximaal 12 maanden
- **Opt-out mogelijkheid**: Inzendingen kunnen worden verwijderd

### 3.4 Subsidiariteit
Er is geen minder ingrijpend alternatief beschikbaar dat hetzelfde doel kan bereiken:
- Anonieme inzending is niet mogelijk vanwege noodzaak tot communicatie
- Papieren proces zou minder efficiënt en minder veilig zijn

---

## 4. Risico-inventarisatie

### 4.1 Identificatie van risico's

| ID | Risico | Impact | Kans | Risicoscore |
|----|--------|--------|------|-------------|
| R1 | Ongeautoriseerde toegang tot documenten | Hoog | Laag | Medium |
| R2 | Datalekken door technische fout | Hoog | Laag | Medium |
| R3 | Overschrijding bewaartermijn | Medium | Medium | Medium |
| R4 | Onvoldoende beveiliging authenticatie | Hoog | Laag | Medium |
| R5 | Bijzondere persoonsgegevens in uploads | Medium | Medium | Medium |
| R6 | Verlies van integriteit audit log | Medium | Laag | Laag |
| R7 | Onvoldoende informatie aan betrokkenen | Laag | Laag | Laag |

### 4.2 Risico-analyse

#### R1: Ongeautoriseerde toegang tot documenten
- **Beschrijving**: Kwaadwillenden krijgen toegang tot geuploade documenten
- **Impact**: Hoog - Vertrouwelijke beleidsdocumenten kunnen worden gelekt
- **Kans**: Laag - Systeem is beveiligd met authenticatie en autorisatie
- **Bestaande maatregelen**:
  - Toegangscontrole op basis van rollen
  - Sessie-gebaseerde authenticatie voor admin portal
  - Documenten alleen toegankelijk via API met slug

#### R2: Datalekken door technische fout
- **Beschrijving**: Bug of misconfiguratie leidt tot onbedoelde blootstelling
- **Impact**: Hoog - Persoonsgegevens kunnen worden blootgesteld
- **Kans**: Laag - Code wordt getest en gereviewed
- **Bestaande maatregelen**:
  - Input validatie op alle endpoints
  - Prepared statements tegen SQL injection
  - CORS configuratie

#### R3: Overschrijding bewaartermijn
- **Beschrijving**: Gegevens worden langer bewaard dan toegestaan
- **Impact**: Medium - Non-compliance met AVG
- **Kans**: Medium - Handmatige processen kunnen falen
- **Bestaande maatregelen**:
  - Bewaartermijn gedocumenteerd in privacyverklaring
  - Audit log van alle verwerkingen

#### R4: Onvoldoende beveiliging authenticatie
- **Beschrijving**: Zwakke wachtwoorden of sessie-hijacking
- **Impact**: Hoog - Ongeautoriseerde toegang tot admin portal
- **Kans**: Laag - Moderne beveiligingsmaatregelen geïmplementeerd
- **Bestaande maatregelen**:
  - Argon2 password hashing
  - HttpOnly, Secure, SameSite cookies
  - Rate limiting op login attempts

#### R5: Bijzondere persoonsgegevens in uploads
- **Beschrijving**: Gebruikers uploaden documenten met bijzondere persoonsgegevens
- **Impact**: Medium - Hogere beschermingseisen van toepassing
- **Kans**: Medium - Niet volledig te controleren wat gebruikers uploaden
- **Bestaande maatregelen**:
  - Classificatiesysteem met restricted optie
  - Waarschuwing bij restricted classificatie

---

## 5. Maatregelen

### 5.1 Technische maatregelen

| Maatregel | Risico's | Status |
|-----------|----------|--------|
| HTTPS (TLS) versleuteling | R1, R2 | Geïmplementeerd |
| Argon2 password hashing | R4 | Geïmplementeerd |
| Rate limiting | R4 | Geïmplementeerd |
| Input validatie | R2 | Geïmplementeerd |
| Secure session cookies | R4 | Geïmplementeerd |
| Audit logging | R6 | Geïmplementeerd |
| Bestandsvalidatie | R5 | Geïmplementeerd |

### 5.2 Organisatorische maatregelen

| Maatregel | Risico's | Status |
|-----------|----------|--------|
| Privacyverklaring | R7 | Opgesteld |
| Verwerkersovereenkomsten | R1 | Te sluiten |
| Procedure gegevensverwijdering | R3 | Te documenteren |
| Incidentprocedure | R1, R2 | Te documenteren |
| Training beheerders | R1, R4 | Te plannen |

### 5.3 Aanvullende maatregelen

Voor verdere risicobeperking worden de volgende aanvullende maatregelen aanbevolen:

1. **Automatische verwijdering**: Implementeer automatische verwijdering van gegevens na 12 maanden
2. **Penetratietest**: Voer voor livegang een penetratietest uit
3. **Backup versleuteling**: Versleutel backups at-rest
4. **Monitoring**: Implementeer alerting bij verdachte activiteiten

---

## 6. Restrisico's

Na implementatie van alle maatregelen resteren de volgende restrisico's:

| Risico | Restrisico | Acceptatie |
|--------|------------|------------|
| R1 | Laag | Acceptabel |
| R2 | Laag | Acceptabel |
| R3 | Laag | Acceptabel met automatische verwijdering |
| R4 | Laag | Acceptabel |
| R5 | Laag | Acceptabel met gebruikerswaarschuwing |
| R6 | Laag | Acceptabel |
| R7 | Laag | Acceptabel |

---

## 7. Advies FG

*Dit onderdeel wordt ingevuld door de Functionaris Gegevensbescherming*

### 7.1 Beoordeling
[ ] Positief advies
[ ] Positief advies met voorwaarden
[ ] Negatief advies

### 7.2 Voorwaarden/Opmerkingen
_Te vullen door FG_

### 7.3 Datum en handtekening
_Te vullen door FG_

---

## 8. Besluit verwerkingsverantwoordelijke

*Dit onderdeel wordt ingevuld door de verwerkingsverantwoordelijke*

### 8.1 Besluit
[ ] Verwerking goedgekeurd
[ ] Verwerking goedgekeurd met voorwaarden
[ ] Verwerking niet goedgekeurd

### 8.2 Motivatie
_Te vullen door verwerkingsverantwoordelijke_

### 8.3 Datum en handtekening
_Te vullen door verwerkingsverantwoordelijke_

---

## 9. Herziening

Deze DPIA wordt herzien:
- Bij significante wijzigingen aan de verwerking
- Na 12 maanden (einde PoC periode)
- Na een beveiligingsincident

---

## Bijlagen

### Bijlage A: Verwerkingsregister
*Referentie naar het verwerkingsregister van MinBZK*

### Bijlage B: Technische architectuur
*Zie projectdocumentatie*

### Bijlage C: Privacyverklaring
*Zie doc/privacy-statement-nl.md*

---

*Document opgesteld conform het Rijksmodel DPIA en de Handleiding DPIA van de Autoriteit Persoonsgegevens*

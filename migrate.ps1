$workspaceRoot = (Resolve-Path -LiteralPath ".").Path
$tempMigrations = Join-Path $workspaceRoot "target\all_migrations"

New-Item -ItemType Directory -Force -Path $tempMigrations | Out-Null
Get-ChildItem -LiteralPath $tempMigrations -File | Remove-Item

Copy-Item -Path ".\migrations\*" -Destination $tempMigrations -Force

Copy-Item -Path ".\crates\rocketcon-db\migrations\*" -Destination $tempMigrations -Force

$duplicateVersions = Get-ChildItem -LiteralPath $tempMigrations -File -Filter "*.sql" |
    Group-Object { ($_.BaseName -split "_", 2)[0] } |
    Where-Object Count -gt 1

if ($duplicateVersions) {
    throw "Duplicate migration versions: $(($duplicateVersions.Name) -join ', ')"
}

cargo sqlx migrate run --source $tempMigrations --database-url "sqlite://database/astronomicon.db"
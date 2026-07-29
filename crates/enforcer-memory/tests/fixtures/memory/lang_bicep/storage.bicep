param location string = 'eastus'

var storageName = 'st${uniqueString(resourceGroup().id)}'

resource storageAccount 'Microsoft.Storage/storageAccounts@2021-09-01' = {
  name: storageName
  location: location
  sku: {
    name: 'Standard_LRS'
  }
  kind: 'StorageV2'
}

func buildName(prefix string, suffix string) string => '${prefix}-${suffix}'

module networkModule 'network.bicep' = {
  name: 'networkDeployment'
  params: {
    location: location
  }
}

output storageId string = storageAccount.id

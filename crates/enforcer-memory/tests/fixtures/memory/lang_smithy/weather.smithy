$version: "2.0"

namespace example.weather

use aws.protocols#restJson1

service Weather {
    version: "2006-03-01"
    resources: [City]
}

resource City {
    identifiers: { cityId: CityId }
}

structure CityId {
    id: String
}

operation GetCity {
    input: GetCityInput
    output: GetCityOutput
}

structure GetCityInput {
    cityId: CityId
}

structure GetCityOutput {
    name: String
}

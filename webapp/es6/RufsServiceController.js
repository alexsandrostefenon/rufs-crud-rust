import {CrudController} from "./CrudController.js";
import {CrudItemJson} from "./CrudItemJson.js";
import {CaseConvert} from "./CaseConvert.js";
import {HttpRestRequest} from "./ServerConnection.js";

class RufsServiceController extends CrudController {

    constructor(serverConnection, $scope) {
    	super(serverConnection, $scope);
    	this.rufsService.label = "OpenApi/Swagger Operations";
		this.properties.operationId.orderIndex = 1;
		this.properties.operationId.sortType = "asc";
		this.properties.path.orderIndex = 2;
		this.properties.path.sortType = "asc";
		this.properties.method.orderIndex = 3;
		this.properties.method.sortType = "asc";
		this.properties.parameter.orderIndex = 4;

    	const schemaProperties = {
    			"hiden":{"type": "boolean", "orderIndex": 1, "sortType": "desc"},
    			// OpenApi / JSON Schema
    			"essential":{"type": "boolean", "orderIndex": 2, "sortType": "asc"},
    			"type":{"options": ["string", "integer", "boolean", "number", "date-time", "date", "time"]},
    			"maxLength":{"type": "integer"},
    			"precision":{"type": "integer"},
    			"scale":{"type": "integer"},
    			"format":{},
    			"pattern":{},
    			"enum": {},
    			"default":{},
    			"example":{},
    			"readOnly":{"type": "boolean"},
    			"$ref":{},
    			"title":{},
    			// DataBase
    			"identityGeneration":{"options": ["ALWAYS", "BY DEFAULT"]},
    			"unique":{},
    			"updatable":{"type": "boolean"},
    			"comment":{},
    			// query
    			"orderIndex":{"type": "integer"},
    			"sortType":{"options": ["asc", "desc"]},
    			// Html Input/Table
    			"enumLabels": {},
    			"tableVisible":{"type": "boolean"},
    			"shortDescription":{"type": "boolean"},
    			"isClonable":{"type": "boolean"},
    			};

    	schemaProperties.$ref.enum = [];
/*    	
    	for (let service of this.rufsService.list) {
    		for (let fieldName of service.primaryKeys) {
				schemaProperties.$ref.enum.push("#/components/schemas/" + service.name);
    		}
    	}
*/
       	this.listItemCrudJson.push(new CrudItemJson(this, schemaProperties, "parameter", "Query String", this.serverConnection));
       	this.listItemCrudJson.push(new CrudItemJson(this, schemaProperties, "requestBody", "Request Body", this.serverConnection));
       	this.listItemCrudJson.push(new CrudItemJson(this, schemaProperties, "response", "Response Ok", this.serverConnection));
    }

    save() {
		this.instance.name =  CaseConvert.underscoreToCamel(this.instance.name);
		return super.save();
    }

}

export {RufsServiceController}

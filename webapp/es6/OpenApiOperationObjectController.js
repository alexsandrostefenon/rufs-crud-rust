import {CrudController} from "./CrudController.js";
import {CrudItemJson} from "./CrudItemJson.js";
import {CaseConvert} from "./CaseConvert.js";
import {HttpRestRequest} from "./ServerConnection.js";
import {ServerConnectionUI} from "./ServerConnectionUI.js";

class OpenApiOperationObjectController extends CrudController {

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

		const options = {disableOrderByName: true};

       	this.listItemCrudJson.push(new CrudItemJson(this, this.properties.parameter.properties, "parameter", "Query String", this.serverConnection, options));
       	this.listItemCrudJson.push(new CrudItemJson(this, this.properties.requestBody.properties, "requestBody", "Request Body", this.serverConnection, options));
       	this.listItemCrudJson.push(new CrudItemJson(this, this.properties.response.properties, "response", "Response Ok", this.serverConnection, options));
    }

    get(primaryKey) {
    	return super.get(primaryKey).
    	then(response => {
			if (response.data != null) {
//    		if (response.data && response.data.response && response.data.response.Itens && response.data.response.Itens.items) response.data.response.Itens.items = response.data.response.Itens.items.properties;
//    		if (response.data && response.data.response && response.data.response.Properties && response.data.response.Properties.properties) response.data.response.Properties = response.data.response.Properties.properties;
			}

    		return response;
    	}).
    	then(response => {
			this.serverConnection.$scope.$apply();
    		return response;
    	});
    }

}

export {OpenApiOperationObjectController}

import { CrudController } from "./CrudController.js";
import { CrudItemJson } from "./CrudItemJson.js";
import { CrudJsonArray } from "./CrudJsonArray.js";
import { CaseConvert } from "./CaseConvert.js";
import { HttpRestRequest } from "./ServerConnection.js";

class UserController extends CrudController {

	constructor(serverConnection, $scope) {
		super(serverConnection, $scope);
		this.properties["password"].htmlType = "password";
//		this.methods = ["get", "post", "put", "delete"];

		if (this.action != "search") {
			this.crudJsonArrayRoles = new CrudJsonArray(this.serverConnection, this.path, this.method, this.schema_place, "roles", null, this, {"title": "Controle de Acesso"});
			// TODO : passar este enum para o Rust
			this.crudJsonArrayRoles.properties["path"].enum = Object.keys(this.serverConnection.openapi.get_paths());
			//this.crudJsonArrayRoles.properties["mask"].type = "string";
			this.listCrudJsonArray.push(this.crudJsonArrayRoles);
			this.listCrudJsonArray.push(new CrudJsonArray(this.serverConnection, this.path, this.method, this.schema_place, "menu", null, this, {"title": "Menu"}));
			this.listCrudJsonArray.push(new CrudJsonArray(this.serverConnection, this.path, this.method, this.schema_place, "routes", null, this, {"title": "Rotas de URL AngularJs"}));
			this.rufsService.params.saveAndExit = false;
		}
	}

	get(primaryKey) {
		return super.get(primaryKey).then(response => {
			this.crudJsonArrayRoles.get(this.instance).then(() => {
				this.crudJsonArrayRoles.paginate({pageSize: 1000})
				this.serverConnection.$scope.$apply();
				return response
			})
		})
	}

	save() {
		this.instance.password = HttpRestRequest.MD5(this.instance.password);
		return super.save();
	}

	update() {
		this.instance.menu = this.instance.menu || [];

		for (let role of this.instance.roles) {
			const oldRole = this.original.roles.find(item => item.path == role.path)

			if (oldRole == undefined) {
				if (this.instance.path == undefined) {
					this.instance.path = `${role.path}/search`;
				}

				const serviceName = CaseConvert.underscoreToCamel(role.path, false)
				const listDependencies = this.serverConnection.getDependencies(serviceName);

				for (let dependency of listDependencies) {
					if (this.instance.roles.find(item => item.path == dependency) == undefined) {
						this.instance.roles.push({
							path: dependency,
//							mask: 0x01
						})
					}
				}

				if (this.instance.menu.find(item => item.label == serviceName) == null) {
					this.instance.menu.push({"group": "services", "label": serviceName,	"path": `${role.path}/search`});
				}
			}
		}

		if (this.instance.password != null && this.instance.password.length < 32) {
			this.instance.password = HttpRestRequest.MD5(this.instance.password);
		}

		return super.update();
	}

}

export { UserController }
